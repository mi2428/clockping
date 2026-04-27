use std::time::Duration;

use crate::{
    event::{ProbeEvent, ProbeOutcome},
    runner::{LossPeriod, Summary},
};

use super::{
    Output,
    style::{
        AnsiStyle, detail_value_style, format_duration, format_duration_ms, loss_count_style,
        loss_percent_style,
    },
};

pub(super) fn build_event(output: &Output, event: &ProbeEvent) -> String {
    let mut line = String::new();
    if let Some(timestamp) = output.timestamp(event.ts) {
        line.push_str(&output.paint(AnsiStyle::Dim, timestamp));
        line.push(' ');
    }
    line.push_str(&output.paint(AnsiStyle::Cyan, event.protocol));
    line.push(' ');
    line.push_str(&output.paint(AnsiStyle::Blue, &event.target));
    line.push_str(" seq=");
    line.push_str(&output.paint(AnsiStyle::Yellow, event.seq.to_string()));

    match &event.outcome {
        ProbeOutcome::Reply {
            rtt,
            peer,
            bytes,
            ttl,
            detail,
        } => {
            line.push(' ');
            line.push_str(&output.paint(AnsiStyle::Green, "reply"));
            push_kv(output, &mut line, "from", peer, AnsiStyle::Blue);
            if let Some(bytes) = bytes {
                push_kv(
                    output,
                    &mut line,
                    "bytes",
                    bytes.to_string(),
                    AnsiStyle::Cyan,
                );
            }
            if let Some(ttl) = ttl {
                push_kv(
                    output,
                    &mut line,
                    "ttl",
                    ttl.to_string(),
                    AnsiStyle::Magenta,
                );
            }
            push_kv(
                output,
                &mut line,
                "rtt",
                format_duration_ms(*rtt),
                AnsiStyle::Green,
            );
            for (key, value) in detail {
                push_kv(
                    output,
                    &mut line,
                    key,
                    value,
                    detail_value_style(key, value),
                );
            }
            if let Some(recovery) = &event.recovery {
                line.push(' ');
                line.push_str(&output.paint(AnsiStyle::Green, "recovered"));
                push_kv(
                    output,
                    &mut line,
                    "loss",
                    recovery.lost.to_string(),
                    AnsiStyle::Red,
                );
                push_kv(
                    output,
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
            line.push_str(&output.paint(AnsiStyle::Yellow, "timeout"));
            for (key, value) in detail {
                push_kv(
                    output,
                    &mut line,
                    key,
                    value,
                    detail_value_style(key, value),
                );
            }
        }
        ProbeOutcome::Error(error) => {
            line.push_str(" error=");
            line.push_str(&output.paint(AnsiStyle::Red, error));
        }
    }

    line
}

pub(super) fn build_summary(output: &Output, summary: &Summary) -> String {
    let mut out = String::new();
    out.push('\n');
    out.push_str("--- ");
    out.push_str(&output.paint(AnsiStyle::Blue, &summary.target));
    out.push(' ');
    out.push_str(&output.paint(AnsiStyle::Bold, "clockping statistics"));
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
    out.push_str(&output.paint(loss_count_style(lost), lost.to_string()));
    out.push_str(" lost, ");
    out.push_str(&output.paint(loss_percent_style(loss_pct), format!("{loss_pct:.1}% loss")));
    out.push('\n');

    if let Some((min, avg, max)) = summary.rtt_min_avg_max() {
        out.push_str("rtt min/avg/max = ");
        out.push_str(&output.paint(AnsiStyle::Green, format_duration_ms(min)));
        out.push('/');
        out.push_str(&output.paint(AnsiStyle::Cyan, format_duration_ms(avg)));
        out.push('/');
        out.push_str(&output.paint(AnsiStyle::Magenta, format_duration_ms(max)));
        out.push('\n');
    }

    if !summary.loss_periods.is_empty() {
        out.push_str("loss periods:\n");
        for period in &summary.loss_periods {
            out.push_str(&format_loss_period(output, period));
            out.push('\n');
        }
    }
    out
}

fn push_kv(output: &Output, out: &mut String, key: &str, value: impl AsRef<str>, style: AnsiStyle) {
    out.push(' ');
    out.push_str(key);
    out.push('=');
    out.push_str(&output.paint(style, value));
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Local;

    use crate::{
        event::ProbeOutcome,
        timefmt::{TimestampFormatter, TimestampKind},
    };

    use super::*;

    #[test]
    fn text_summary_renders_as_contiguous_block() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 3;
        summary.received = 2;
        summary.rtts = vec![Duration::from_millis(10), Duration::from_millis(20)];

        let output = Output::new(
            TimestampFormatter::new(TimestampKind::None, None),
            false,
            false,
        );
        let text = build_summary(&output, &summary);

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
            TimestampFormatter::new(TimestampKind::None, None),
            false,
            true,
        );

        let text = build_event(&output, &event);

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
            TimestampFormatter::new(TimestampKind::None, None),
            false,
            true,
        );

        let text = build_summary(&output, &summary);

        assert!(text.contains("--- \x1b[34mtarget\x1b[0m \x1b[1mclockping statistics\x1b[0m ---"));
        assert!(text.contains("\x1b[32m0.0% loss\x1b[0m"));
        assert!(text.contains(
            "rtt min/avg/max = \x1b[32m5.000ms\x1b[0m/\x1b[36m6.000ms\x1b[0m/\x1b[35m7.000ms\x1b[0m"
        ));
    }
}
