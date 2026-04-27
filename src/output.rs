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
    colored: bool,
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
    pub fn new(timestamps: TimestampFormatter, json: bool, colored: bool) -> Self {
        Self {
            timestamps,
            json,
            colored: colored && !json,
        }
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
            Some(timestamp) => {
                write_stdout_line(format!("{} {line}", self.paint(AnsiStyle::Dim, timestamp)))?
            }
            None => write_stdout_line(line)?,
        }
        Ok(())
    }

    pub fn print_external_line_without_timestamp(
        &self,
        stream: &'static str,
        line: &str,
    ) -> anyhow::Result<()> {
        if self.json {
            write_stdout_line(&serde_json::to_string(&JsonExternalLine {
                ts: String::new(),
                stream,
                line,
            })?)?;
            return Ok(());
        }

        write_stdout_line(line)
    }

    pub fn print_external_stderr_line(&self, line: &str) -> anyhow::Result<()> {
        write_stderr_line(line)
    }

    pub fn print_event(&self, event: &ProbeEvent) -> anyhow::Result<()> {
        if self.json {
            let timestamp = self.timestamp(event.ts).unwrap_or_default();
            write_stdout_line(serde_json::to_string(&event.as_json(timestamp))?)?;
            return Ok(());
        }

        write_stdout_line(self.build_text_event(event))?;
        Ok(())
    }

    fn build_text_event(&self, event: &ProbeEvent) -> String {
        let mut line = String::new();
        if let Some(timestamp) = self.timestamp(event.ts) {
            line.push_str(&self.paint(AnsiStyle::Dim, timestamp));
            line.push(' ');
        }
        line.push_str(&self.paint(AnsiStyle::Cyan, event.protocol));
        line.push(' ');
        line.push_str(&self.paint(AnsiStyle::Blue, &event.target));
        line.push_str(" seq=");
        line.push_str(&self.paint(AnsiStyle::Yellow, event.seq.to_string()));

        match &event.outcome {
            ProbeOutcome::Reply {
                rtt,
                peer,
                bytes,
                ttl,
                detail,
            } => {
                line.push(' ');
                line.push_str(&self.paint(AnsiStyle::Green, "reply"));
                self.push_kv(&mut line, "from", peer, AnsiStyle::Blue);
                if let Some(bytes) = bytes {
                    self.push_kv(&mut line, "bytes", bytes.to_string(), AnsiStyle::Cyan);
                }
                if let Some(ttl) = ttl {
                    self.push_kv(&mut line, "ttl", ttl.to_string(), AnsiStyle::Magenta);
                }
                self.push_kv(&mut line, "rtt", format_duration_ms(*rtt), AnsiStyle::Green);
                for (key, value) in detail {
                    self.push_kv(&mut line, key, value, detail_value_style(key, value));
                }
                if let Some(recovery) = &event.recovery {
                    line.push(' ');
                    line.push_str(&self.paint(AnsiStyle::Green, "recovered"));
                    self.push_kv(&mut line, "loss", recovery.lost.to_string(), AnsiStyle::Red);
                    self.push_kv(
                        &mut line,
                        "duration",
                        format_duration(Duration::from_millis(
                            recovery.duration_ms.min(u128::from(u64::MAX)) as u64,
                        )),
                        AnsiStyle::Yellow,
                    );
                }
            }
            ProbeOutcome::Timeout { detail } => {
                line.push(' ');
                line.push_str(&self.paint(AnsiStyle::Yellow, "timeout"));
                for (key, value) in detail {
                    self.push_kv(&mut line, key, value, detail_value_style(key, value));
                }
            }
            ProbeOutcome::Error(error) => {
                line.push_str(" error=");
                line.push_str(&self.paint(AnsiStyle::Red, error));
            }
        }

        line
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
        out.push_str("--- ");
        out.push_str(&self.paint(AnsiStyle::Blue, &summary.target));
        out.push(' ');
        out.push_str(&self.paint(AnsiStyle::Bold, "clockping statistics"));
        out.push_str(" ---\n");

        let lost = summary.sent.saturating_sub(summary.received);
        let loss_pct = if summary.sent == 0 {
            0.0
        } else {
            lost as f64 / summary.sent as f64 * 100.0
        };
        out.push_str(&summary.sent.to_string());
        out.push_str(" probes transmitted, ");
        out.push_str(&summary.received.to_string());
        out.push_str(" replies received, ");
        out.push_str(&self.paint(loss_count_style(lost), lost.to_string()));
        out.push_str(" lost, ");
        out.push_str(&self.paint(loss_percent_style(loss_pct), format!("{loss_pct:.1}% loss")));
        out.push('\n');

        if let Some((min, avg, max)) = summary.rtt_min_avg_max() {
            out.push_str("rtt min/avg/max = ");
            out.push_str(&self.paint(AnsiStyle::Green, format_duration_ms(min)));
            out.push('/');
            out.push_str(&self.paint(AnsiStyle::Cyan, format_duration_ms(avg)));
            out.push('/');
            out.push_str(&self.paint(AnsiStyle::Magenta, format_duration_ms(max)));
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

    fn push_kv(&self, out: &mut String, key: &str, value: impl AsRef<str>, style: AnsiStyle) {
        out.push(' ');
        out.push_str(key);
        out.push('=');
        out.push_str(&self.paint(style, value));
    }

    fn paint(&self, style: AnsiStyle, text: impl AsRef<str>) -> String {
        paint(self.colored, style, text)
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

#[derive(Clone, Copy, Debug)]
enum AnsiStyle {
    Bold,
    Dim,
    Blue,
    Cyan,
    Green,
    Yellow,
    Magenta,
    Red,
}

impl AnsiStyle {
    const fn code(self) -> &'static str {
        match self {
            Self::Bold => "1",
            Self::Dim => "2",
            Self::Blue => "34",
            Self::Cyan => "36",
            Self::Green => "32",
            Self::Yellow => "33",
            Self::Magenta => "35",
            Self::Red => "31",
        }
    }
}

fn paint(enabled: bool, style: AnsiStyle, text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    if enabled {
        format!("\x1b[{}m{text}\x1b[0m", style.code())
    } else {
        text.to_string()
    }
}

fn detail_value_style(key: &str, value: &str) -> AnsiStyle {
    match key {
        "icmp_seq" | "gtp_seq" => AnsiStyle::Yellow,
        "ttl" => AnsiStyle::Magenta,
        "status" if value.starts_with('2') || value.starts_with('3') => AnsiStyle::Green,
        "status" => AnsiStyle::Red,
        "method" | "version" | "url" => AnsiStyle::Cyan,
        _ => AnsiStyle::Blue,
    }
}

fn loss_count_style(lost: u64) -> AnsiStyle {
    if lost == 0 {
        AnsiStyle::Green
    } else {
        AnsiStyle::Red
    }
}

fn loss_percent_style(loss_pct: f64) -> AnsiStyle {
    if loss_pct == 0.0 {
        AnsiStyle::Green
    } else if loss_pct < 100.0 {
        AnsiStyle::Yellow
    } else {
        AnsiStyle::Red
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

fn write_stderr_line(line: impl AsRef<str>) -> anyhow::Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{}", line.as_ref())?;
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
            false,
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
            false,
        );
        let text = output.build_text_summary(&summary);

        assert!(text.starts_with("\n--- target clockping statistics ---\n"));
        assert!(text.contains(
            "3 probes transmitted, 2 replies received, 1 lost, 33.3% loss\nrtt min/avg/max = 10.000ms/15.000ms/20.000ms\n"
        ));
    }

    #[test]
    fn colored_event_highlights_target_status_and_rtt() {
        let event = ProbeEvent {
            ts: Local::now(),
            protocol: "icmp",
            target: "1.1.1.1 (1.1.1.1)".to_string(),
            seq: 4,
            outcome: ProbeOutcome::Reply {
                rtt: Duration::from_micros(5903),
                peer: "1.1.1.1".to_string(),
                bytes: Some(64),
                ttl: Some(58),
                detail: vec![("icmp_seq".to_string(), "4".to_string())],
            },
            recovery: None,
        };
        let output = Output::new(
            TimestampFormatter::new(crate::timefmt::TimestampKind::None, None),
            false,
            true,
        );

        let text = output.build_text_event(&event);

        assert!(text.contains("\x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m"));
        assert!(text.contains("seq=\x1b[33m4\x1b[0m"));
        assert!(text.contains("\x1b[32mreply\x1b[0m"));
        assert!(text.contains("rtt=\x1b[32m5.903ms\x1b[0m"));
    }

    #[test]
    fn colored_summary_highlights_loss_and_rtt() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 2;
        summary.received = 2;
        summary.rtts = vec![Duration::from_millis(5), Duration::from_millis(7)];
        let output = Output::new(
            TimestampFormatter::new(crate::timefmt::TimestampKind::None, None),
            false,
            true,
        );

        let text = output.build_text_summary(&summary);

        assert!(text.contains("--- \x1b[34mtarget\x1b[0m \x1b[1mclockping statistics\x1b[0m ---"));
        assert!(text.contains("\x1b[32m0.0% loss\x1b[0m"));
        assert!(text.contains(
            "rtt min/avg/max = \x1b[32m5.000ms\x1b[0m/\x1b[36m6.000ms\x1b[0m/\x1b[35m7.000ms\x1b[0m"
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
