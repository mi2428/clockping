use serde::Serialize;

use crate::{
    event::ProbeEvent,
    runner::{LossPeriod, Summary},
};

use super::{Output, writer::write_stdout_line};

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

pub(super) fn print_external_line(
    _output: &Output,
    stream: &'static str,
    line: &str,
    ts: String,
) -> anyhow::Result<()> {
    write_stdout_line(serde_json::to_string(&JsonExternalLine {
        ts,
        stream,
        line,
    })?)
}

pub(super) fn print_event(output: &Output, event: &ProbeEvent) -> anyhow::Result<()> {
    let timestamp = output.timestamp(event.ts).unwrap_or_default();
    write_stdout_line(serde_json::to_string(&event.as_json(timestamp))?)
}

pub(super) fn print_summary(output: &Output, summary: &Summary) -> anyhow::Result<()> {
    write_stdout_line(serde_json::to_string(&build_summary(output, summary))?)
}

fn build_summary(output: &Output, summary: &Summary) -> JsonSummary {
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
            .map(|period| build_loss_period(output, period))
            .collect(),
    }
}

fn build_loss_period(output: &Output, period: &LossPeriod) -> JsonLossPeriod {
    let start = output
        .timestamp(period.start)
        .unwrap_or_else(|| period.start.to_rfc3339());
    let (end, duration_ms) = period.end.map_or((None, None), |end| {
        let end_text = output.timestamp(end).unwrap_or_else(|| end.to_rfc3339());
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::timefmt::{TimestampFormatter, TimestampKind};

    use super::*;

    #[test]
    fn json_summary_includes_stats() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 3;
        summary.received = 2;
        summary.rtts = vec![Duration::from_millis(10), Duration::from_millis(20)];

        let output = Output::new(
            TimestampFormatter::new(TimestampKind::None, None),
            true,
            false,
        );
        let value = serde_json::to_value(build_summary(&output, &summary)).unwrap();

        assert_eq!(value["type"], "summary");
        assert_eq!(value["target"], "target");
        assert_eq!(value["sent"], 3);
        assert_eq!(value["received"], 2);
        assert_eq!(value["lost"], 1);
        assert_eq!(value["rtt_min_ms"], 10.0);
        assert_eq!(value["rtt_avg_ms"], 15.0);
        assert_eq!(value["rtt_max_ms"], 20.0);
    }
}
