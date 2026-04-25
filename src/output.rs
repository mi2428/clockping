use std::time::Duration;

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
            println!(
                "{}",
                serde_json::to_string(&JsonExternalLine {
                    ts: timestamp,
                    stream,
                    line
                })?
            );
            return Ok(());
        }

        match self.timestamp(ts) {
            Some(timestamp) => println!("{timestamp} {line}"),
            None => println!("{line}"),
        }
        Ok(())
    }

    pub fn print_event(&self, event: &ProbeEvent) -> anyhow::Result<()> {
        if self.json {
            let timestamp = self.timestamp(event.ts).unwrap_or_default();
            println!("{}", serde_json::to_string(&event.as_json(timestamp))?);
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
            ProbeOutcome::Timeout => line.push_str(" timeout"),
            ProbeOutcome::Error(error) => {
                line.push_str(" error=");
                line.push_str(error);
            }
        }

        println!("{line}");
        Ok(())
    }

    pub fn print_summary(&self, summary: &Summary) {
        if self.json {
            return;
        }

        println!();
        println!("--- {} clockping statistics ---", summary.target);
        let lost = summary.sent.saturating_sub(summary.received);
        let loss_pct = if summary.sent == 0 {
            0.0
        } else {
            lost as f64 / summary.sent as f64 * 100.0
        };
        println!(
            "{} probes transmitted, {} replies received, {} lost, {:.1}% loss",
            summary.sent, summary.received, lost, loss_pct
        );

        if let Some((min, avg, max)) = summary.rtt_min_avg_max() {
            println!(
                "rtt min/avg/max = {}/{}/{}",
                format_duration_ms(min),
                format_duration_ms(avg),
                format_duration_ms(max)
            );
        }

        if !summary.loss_periods.is_empty() {
            println!("loss periods:");
            for period in &summary.loss_periods {
                print_loss_period(self, period);
            }
        }
    }
}

fn print_loss_period(output: &Output, period: &LossPeriod) {
    let start = output
        .timestamp(period.start)
        .unwrap_or_else(|| period.start.to_rfc3339());
    match period.end {
        Some(end) => {
            let end_text = output.timestamp(end).unwrap_or_else(|| end.to_rfc3339());
            let duration = end.signed_duration_since(period.start).to_std().ok();
            println!(
                "  {} - {}  lost={} duration={}",
                start,
                end_text,
                period.lost,
                duration
                    .map(format_duration)
                    .unwrap_or_else(|| "n/a".to_string())
            );
        }
        None => {
            println!("  {} - ongoing  lost={}", start, period.lost);
        }
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
}
