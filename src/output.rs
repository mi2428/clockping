use std::{
    io::{self, Write},
    time::Duration,
};

use chrono::{DateTime, Local};
use serde::Serialize;

use crate::{
    event::{ProbeEvent, ProbeOutcome},
    runner::{LossPeriod, Summary},
    timefmt::TimestampFormatter,
};

#[derive(Clone, Debug)]
pub struct Output {
    timestamps: TimestampFormatter,
    json: bool,
}

#[derive(Debug, Serialize)]
struct JsonExternalLine<'a> {
    ts: String,
    stream: &'static str,
    line: &'a str,
}

#[derive(Debug, Serialize)]
struct JsonSummary {
    r#type: &'static str,
    target: String,
    sent: u64,
    received: u64,
    lost: u64,
    loss_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtt_min_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtt_avg_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtt_max_ms: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    loss_periods: Vec<JsonLossPeriod>,
}

#[derive(Debug, Serialize)]
struct JsonLossPeriod {
    start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<String>,
    lost: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u64>,
}

impl Output {
    pub fn new(timestamps: TimestampFormatter, json: bool) -> Self {
        Self { timestamps, json }
    }

    pub fn timestamp(&self, ts: DateTime<Local>) -> Option<String> {
        self.timestamps.format(ts)
    }

    pub fn print_external_line(&self, stream: &'static str, line: &str) -> anyhow::Result<()> {
        let ts = Local::now();
        if self.json {
            let timestamp = self.timestamp(ts).unwrap_or_default();
            write_stdout_line(&serde_json::to_string(&JsonExternalLine {
                ts: timestamp,
                stream,
                line,
            })?)?;
            return Ok(());
        }

        match self.timestamp(ts) {
            Some(timestamp) => write_stdout_line(format!("{timestamp} {line}"))?,
            None => write_stdout_line(line)?,
        }
        Ok(())
    }

    pub fn print_event(&self, event: &ProbeEvent) -> anyhow::Result<()> {
        if self.json {
            let timestamp = self.timestamp(event.ts).unwrap_or_default();
            write_stdout_line(serde_json::to_string(&event.as_json(timestamp))?)?;
            return Ok(());
        }

        let mut line = String::new();
        if let Some(timestamp) = self.timestamp(event.ts) {
            line.push_str(&timestamp);
            line.push(' ');
        }
        line.push_str(event.protocol);
        line.push(' ');
        line.push_str(&event.target);
        line.push_str(" seq=");
        line.push_str(&event.seq.to_string());

        match &event.outcome {
            ProbeOutcome::Reply {
                rtt,
                peer,
                bytes,
                ttl,
                detail,
            } => {
                line.push_str(" reply");
                line.push_str(" from=");
                line.push_str(peer);
                if let Some(bytes) = bytes {
                    line.push_str(" bytes=");
                    line.push_str(&bytes.to_string());
                }
                if let Some(ttl) = ttl {
                    line.push_str(" ttl=");
                    line.push_str(&ttl.to_string());
                }
                line.push_str(" rtt=");
                line.push_str(&format_duration_ms(*rtt));
                for (key, value) in detail {
                    line.push(' ');
                    line.push_str(key);
                    line.push('=');
                    line.push_str(value);
                }
                if let Some(recovery) = &event.recovery {
                    line.push_str(" recovered loss=");
                    line.push_str(&recovery.lost.to_string());
                    line.push_str(" duration=");
                    line.push_str(&format_duration(Duration::from_millis(
                        recovery.duration_ms.min(u128::from(u64::MAX)) as u64,
                    )));
                }
            }
            ProbeOutcome::Timeout { detail } => {
                line.push_str(" timeout");
                for (key, value) in detail {
                    line.push(' ');
                    line.push_str(key);
                    line.push('=');
                    line.push_str(value);
                }
            }
            ProbeOutcome::Error(error) => {
                line.push_str(" error=");
                line.push_str(error);
            }
        }

        write_stdout_line(line)?;
        Ok(())
    }

    pub fn print_summary(&self, summary: &Summary, quiet: bool) -> anyhow::Result<()> {
        if self.json {
            if quiet {
                write_stdout_line(serde_json::to_string(&self.build_json_summary(summary))?)?;
            }
            return Ok(());
        }

        write_stdout_block(&self.build_text_summary(summary))?;
        Ok(())
    }

    fn build_text_summary(&self, summary: &Summary) -> String {
        let mut out = String::new();
        out.push('\n');
        out.push_str(&format!(
            "--- {} clockping statistics ---\n",
            summary.target
        ));

        let lost = summary.sent.saturating_sub(summary.received);
        let loss_pct = if summary.sent == 0 {
            0.0
        } else {
            lost as f64 / summary.sent as f64 * 100.0
        };
        out.push_str(&format!(
            "{} probes transmitted, {} replies received, {} lost, {:.1}% loss",
            summary.sent, summary.received, lost, loss_pct
        ));
        out.push('\n');

        if let Some((min, avg, max)) = summary.rtt_min_avg_max() {
            out.push_str(&format!(
                "rtt min/avg/max = {}/{}/{}",
                format_duration_ms(min),
                format_duration_ms(avg),
                format_duration_ms(max)
            ));
            out.push('\n');
        }

        if !summary.loss_periods.is_empty() {
            out.push_str("loss periods:\n");
            for period in &summary.loss_periods {
                out.push_str(&format_loss_period(self, period));
                out.push('\n');
            }
        }
        out
    }

    fn build_json_summary(&self, summary: &Summary) -> JsonSummary {
        let lost = summary.sent.saturating_sub(summary.received);
        let loss_pct = if summary.sent == 0 {
            0.0
        } else {
            lost as f64 / summary.sent as f64 * 100.0
        };
        let (rtt_min_ms, rtt_avg_ms, rtt_max_ms) = summary
            .rtt_min_avg_max()
            .map(|(min, avg, max)| {
                (
                    Some(min.as_secs_f64() * 1000.0),
                    Some(avg.as_secs_f64() * 1000.0),
                    Some(max.as_secs_f64() * 1000.0),
                )
            })
            .unwrap_or((None, None, None));

        JsonSummary {
            r#type: "summary",
            target: summary.target.clone(),
            sent: summary.sent,
            received: summary.received,
            lost,
            loss_pct,
            rtt_min_ms,
            rtt_avg_ms,
            rtt_max_ms,
            loss_periods: summary
                .loss_periods
                .iter()
                .map(|period| self.build_json_loss_period(period))
                .collect(),
        }
    }

    fn build_json_loss_period(&self, period: &LossPeriod) -> JsonLossPeriod {
        let start = self
            .timestamp(period.start)
            .unwrap_or_else(|| period.start.to_rfc3339());
        let (end, duration_ms) = period.end.map_or((None, None), |end| {
            let end_text = self.timestamp(end).unwrap_or_else(|| end.to_rfc3339());
            let duration_ms = end
                .signed_duration_since(period.start)
                .num_milliseconds()
                .max(0) as u64;
            (Some(end_text), Some(duration_ms))
        });

        JsonLossPeriod {
            start,
            end,
            lost: period.lost,
            duration_ms,
        }
    }
}

fn format_loss_period(output: &Output, period: &LossPeriod) -> String {
    let start = output
        .timestamp(period.start)
        .unwrap_or_else(|| period.start.to_rfc3339());
    match period.end {
        Some(end) => {
            let end_text = output.timestamp(end).unwrap_or_else(|| end.to_rfc3339());
            let duration = end.signed_duration_since(period.start).to_std().ok();
            format!(
                "  {} - {}  lost={} duration={}",
                start,
                end_text,
                period.lost,
                duration
                    .map(format_duration)
                    .unwrap_or_else(|| "n/a".to_string())
            )
        }
        None => format!("  {} - ongoing  lost={}", start, period.lost),
    }
}

pub fn format_duration_ms(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
}

pub fn format_duration(duration: Duration) -> String {
    if duration.as_secs() == 0 {
        return format_duration_ms(duration);
    }
    format!("{:.3}s", duration.as_secs_f64())
}

fn write_stdout_line(line: impl AsRef<str>) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", line.as_ref())?;
    Ok(())
}

fn write_stdout_block(block: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(block.as_bytes())?;
    Ok(())
}

pub fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_subsecond_duration_as_ms() {
        assert_eq!(format_duration(Duration::from_millis(250)), "250.000ms");
    }

    #[test]
    fn format_second_duration_as_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1250)), "1.250s");
    }

    #[test]
    fn json_summary_includes_stats() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 3;
        summary.received = 2;
        summary.rtts = vec![Duration::from_millis(10), Duration::from_millis(20)];

        let output = Output::new(
            TimestampFormatter::new(crate::timefmt::TimestampKind::None, None),
            true,
        );
        let value = serde_json::to_value(output.build_json_summary(&summary)).unwrap();

        assert_eq!(value["type"], "summary");
        assert_eq!(value["target"], "target");
        assert_eq!(value["sent"], 3);
        assert_eq!(value["received"], 2);
        assert_eq!(value["lost"], 1);
        assert_eq!(value["rtt_min_ms"], 10.0);
        assert_eq!(value["rtt_avg_ms"], 15.0);
        assert_eq!(value["rtt_max_ms"], 20.0);
    }

    #[test]
    fn text_summary_renders_as_contiguous_block() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 3;
        summary.received = 2;
        summary.rtts = vec![Duration::from_millis(10), Duration::from_millis(20)];

        let output = Output::new(
            TimestampFormatter::new(crate::timefmt::TimestampKind::None, None),
            false,
        );
        let text = output.build_text_summary(&summary);

        assert!(text.starts_with("\n--- target clockping statistics ---\n"));
        assert!(text.contains(
            "3 probes transmitted, 2 replies received, 1 lost, 33.3% loss\nrtt min/avg/max = 10.000ms/15.000ms/20.000ms\n"
        ));
    }

    #[test]
    fn broken_pipe_errors_are_detected() {
        let error = anyhow::Error::from(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "stdout closed",
        ));

        assert!(is_broken_pipe(&error));
    }
}
