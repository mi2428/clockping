use crate::metrics::{ProbeMetrics, WindowMetrics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrometheusEncoder {
    metric_prefix: String,
    labels: Vec<(String, String)>,
}

impl PrometheusEncoder {
    pub const DEFAULT_PREFIX: &'static str = "clockping";

    #[allow(dead_code)]
    pub fn new(metric_prefix: impl Into<String>) -> anyhow::Result<Self> {
        Self::with_labels(metric_prefix, std::iter::empty::<(String, String)>())
    }

    pub fn with_labels<I, K, V>(metric_prefix: impl Into<String>, labels: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let metric_prefix = metric_prefix.into();
        let labels = labels
            .into_iter()
            .map(|(name, value)| (name.into(), value.into()))
            .collect::<Vec<_>>();
        validate_metric_prefix(&metric_prefix)?;
        validate_prometheus_labels(&labels)?;
        Ok(Self {
            metric_prefix,
            labels,
        })
    }

    pub fn encode_interval(&self, metrics: &ProbeMetrics) -> String {
        render_interval_prometheus_with_labels(metrics, &self.metric_prefix, &self.labels)
    }

    pub fn encode_intervals(&self, metrics: &[ProbeMetrics]) -> String {
        render_interval_prometheus_many_with_labels(metrics, &self.metric_prefix, &self.labels)
    }

    pub fn encode_window(&self, metrics: &WindowMetrics) -> String {
        render_window_prometheus_with_labels(metrics, &self.metric_prefix, &self.labels)
    }
}

impl Default for PrometheusEncoder {
    fn default() -> Self {
        Self {
            metric_prefix: Self::DEFAULT_PREFIX.to_owned(),
            labels: Vec::new(),
        }
    }
}

pub(crate) fn render_interval_prometheus(metrics: &ProbeMetrics, prefix: &str) -> String {
    render_interval_prometheus_with_labels(metrics, prefix, &[])
}

pub(crate) fn render_interval_prometheus_many(metrics: &[ProbeMetrics], prefix: &str) -> String {
    render_interval_prometheus_many_with_labels(metrics, prefix, &[])
}

fn render_interval_prometheus_many_with_labels(
    metrics: &[ProbeMetrics],
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    let mut out = String::new();
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_timestamp_seconds"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.timestamp_unix_seconds,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_sequence"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.seq as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_sent"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.sent as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_received"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.received as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_lost"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.lost as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_loss_percent"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.loss_pct,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_up"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.up,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_status"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set_with_status(
                    labels,
                    &metrics.protocol,
                    &metrics.target,
                    metrics.status,
                ),
                1.0,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_rtt_seconds"),
        metrics.iter().filter_map(|metrics| {
            metrics.rtt_seconds.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    value,
                )
            })
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_bytes"),
        metrics.iter().filter_map(|metrics| {
            metrics.bytes.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    value as f64,
                )
            })
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "probe_ttl"),
        metrics.iter().filter_map(|metrics| {
            metrics.ttl.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    f64::from(value),
                )
            })
        }),
    );
    out
}

pub(crate) fn render_window_prometheus(metrics: &WindowMetrics, prefix: &str) -> String {
    render_window_prometheus_with_labels(metrics, prefix, &[])
}

pub(crate) fn render_window_prometheus_many(metrics: &[WindowMetrics], prefix: &str) -> String {
    render_window_prometheus_many_with_labels(metrics, prefix, &[])
}

fn render_window_prometheus_many_with_labels(
    metrics: &[WindowMetrics],
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    let mut out = String::new();
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_timestamp_seconds"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.timestamp_unix_seconds,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_duration_seconds"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.duration_seconds,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_samples"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.samples as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_replies"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.replies as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_lost"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.lost as f64,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_loss_percent"),
        metrics.iter().map(|metrics| {
            (
                metric_label_set(labels, &metrics.protocol, &metrics.target),
                metrics.loss_pct,
            )
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_rtt_mean_seconds"),
        metrics.iter().filter_map(|metrics| {
            metrics.rtt_mean_seconds.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    value,
                )
            })
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_rtt_min_seconds"),
        metrics.iter().filter_map(|metrics| {
            metrics.rtt_min_seconds.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    value,
                )
            })
        }),
    );
    gauge_family(
        &mut out,
        &metric_name(prefix, "window_rtt_max_seconds"),
        metrics.iter().filter_map(|metrics| {
            metrics.rtt_max_seconds.map(|value| {
                (
                    metric_label_set(labels, &metrics.protocol, &metrics.target),
                    value,
                )
            })
        }),
    );
    out
}

pub(crate) fn validate_metric_prefix(prefix: &str) -> anyhow::Result<()> {
    if !is_valid_metric_name(prefix) {
        anyhow::bail!("invalid Prometheus metric prefix '{prefix}'");
    }
    Ok(())
}

pub(crate) fn is_valid_label_name(name: &str) -> bool {
    is_valid_metric_name(name)
}

fn validate_prometheus_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (name, value) in labels {
        if !is_valid_label_name(name) {
            anyhow::bail!("invalid Prometheus label name '{name}'");
        }
        if value.is_empty() {
            anyhow::bail!("Prometheus label value for '{name}' must not be empty");
        }
    }
    reject_duplicate_labels(labels)
}

pub(crate) fn reject_duplicate_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (index, (name, _)) in labels.iter().enumerate() {
        if labels[..index]
            .iter()
            .any(|(previous_name, _)| previous_name == name)
        {
            anyhow::bail!("duplicate Prometheus label name '{name}'");
        }
    }
    Ok(())
}

fn is_valid_metric_name(name: &str) -> bool {
    let Some((&first, rest)) = name.as_bytes().split_first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    rest.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

fn render_interval_prometheus_with_labels(
    metrics: &ProbeMetrics,
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    let mut out = String::new();
    let label_set = metric_label_set(labels, &metrics.protocol, &metrics.target);
    let status_label_set =
        metric_label_set_with_status(labels, &metrics.protocol, &metrics.target, metrics.status);

    gauge(
        &mut out,
        &metric_name(prefix, "probe_timestamp_seconds"),
        metrics.timestamp_unix_seconds,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_sequence"),
        metrics.seq as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_sent"),
        metrics.sent as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_received"),
        metrics.received as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_lost"),
        metrics.lost as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_loss_percent"),
        metrics.loss_pct,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_up"),
        metrics.up,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "probe_status"),
        1.0,
        &status_label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "probe_rtt_seconds"),
        metrics.rtt_seconds,
        &label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "probe_bytes"),
        metrics.bytes.map(|value| value as f64),
        &label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "probe_ttl"),
        metrics.ttl.map(f64::from),
        &label_set,
    );
    out
}

fn render_window_prometheus_with_labels(
    metrics: &WindowMetrics,
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    let mut out = String::new();
    let label_set = metric_label_set(labels, &metrics.protocol, &metrics.target);

    gauge(
        &mut out,
        &metric_name(prefix, "window_timestamp_seconds"),
        metrics.timestamp_unix_seconds,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "window_duration_seconds"),
        metrics.duration_seconds,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "window_samples"),
        metrics.samples as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "window_replies"),
        metrics.replies as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "window_lost"),
        metrics.lost as f64,
        &label_set,
    );
    gauge(
        &mut out,
        &metric_name(prefix, "window_loss_percent"),
        metrics.loss_pct,
        &label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "window_rtt_mean_seconds"),
        metrics.rtt_mean_seconds,
        &label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "window_rtt_min_seconds"),
        metrics.rtt_min_seconds,
        &label_set,
    );
    gauge_option(
        &mut out,
        &metric_name(prefix, "window_rtt_max_seconds"),
        metrics.rtt_max_seconds,
        &label_set,
    );
    out
}

fn metric_label_set(labels: &[(String, String)], protocol: &str, target: &str) -> String {
    let mut combined = labels.to_vec();
    combined.push(("protocol".to_owned(), protocol.to_owned()));
    combined.push(("target".to_owned(), target.to_owned()));
    label_set(&combined)
}

fn metric_label_set_with_status(
    labels: &[(String, String)],
    protocol: &str,
    target: &str,
    status: &str,
) -> String {
    let mut combined = labels.to_vec();
    combined.push(("protocol".to_owned(), protocol.to_owned()));
    combined.push(("target".to_owned(), target.to_owned()));
    combined.push(("status".to_owned(), status.to_owned()));
    label_set(&combined)
}

fn metric_name(prefix: &str, suffix: &str) -> String {
    format!("{prefix}_{suffix}")
}

fn label_set(labels: &[(String, String)]) -> String {
    if labels.is_empty() {
        return String::new();
    }

    let mut out = String::from("{");
    for (index, (name, value)) in labels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(name);
        out.push_str("=\"");
        push_escaped_label_value(&mut out, value);
        out.push('"');
    }
    out.push('}');
    out
}

fn push_escaped_label_value(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            _ => out.push(ch),
        }
    }
}

fn gauge(out: &mut String, name: &str, value: f64, label_set: &str) {
    out.push_str("# TYPE ");
    out.push_str(name);
    out.push_str(" gauge\n");
    gauge_sample(out, name, value, label_set);
}

fn gauge_sample(out: &mut String, name: &str, value: f64, label_set: &str) {
    out.push_str(name);
    out.push_str(label_set);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn gauge_family(out: &mut String, name: &str, samples: impl IntoIterator<Item = (String, f64)>) {
    let mut samples = samples.into_iter().peekable();
    if samples.peek().is_none() {
        return;
    }
    out.push_str("# TYPE ");
    out.push_str(name);
    out.push_str(" gauge\n");
    for (label_set, value) in samples {
        gauge_sample(out, name, value, &label_set);
    }
}

fn gauge_option(out: &mut String, name: &str, value: Option<f64>, label_set: &str) {
    if let Some(value) = value {
        gauge(out, name, value, label_set);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics() -> ProbeMetrics {
        ProbeMetrics {
            timestamp_unix_seconds: 1.0,
            protocol: "tcp".to_owned(),
            target: "example.com:443".to_owned(),
            seq: 7,
            status: "reply",
            sent: 8,
            received: 6,
            lost: 2,
            loss_pct: 25.0,
            up: 1.0,
            rtt_seconds: Some(0.012),
            bytes: Some(64),
            ttl: Some(58),
        }
    }

    #[test]
    fn interval_metrics_render_prometheus_gauges() {
        let rendered = PrometheusEncoder::with_labels("nettest", [("site", "ci")])
            .unwrap()
            .encode_interval(&sample_metrics());

        assert!(rendered.contains(
            "nettest_probe_sent{site=\"ci\",protocol=\"tcp\",target=\"example.com:443\"} 8\n"
        ));
        assert!(rendered.contains(
            "nettest_probe_status{site=\"ci\",protocol=\"tcp\",target=\"example.com:443\",status=\"reply\"} 1\n"
        ));
        assert!(rendered.contains("nettest_probe_rtt_seconds"));
        assert!(!rendered.contains("clockping_probe_sent"));
    }

    #[test]
    fn interval_metrics_render_multiple_targets_in_one_family() {
        let first = sample_metrics();
        let mut second = sample_metrics();
        second.target = "example.org:443".to_owned();
        second.seq = 8;

        let rendered = PrometheusEncoder::with_labels("nettest", [("site", "ci")])
            .unwrap()
            .encode_intervals(&[first, second]);

        assert_eq!(
            rendered.matches("# TYPE nettest_probe_sent gauge").count(),
            1
        );
        assert!(rendered.contains(
            "nettest_probe_sequence{site=\"ci\",protocol=\"tcp\",target=\"example.com:443\"} 7\n"
        ));
        assert!(rendered.contains(
            "nettest_probe_sequence{site=\"ci\",protocol=\"tcp\",target=\"example.org:443\"} 8\n"
        ));
    }

    #[test]
    fn window_metrics_render_prometheus_gauges() {
        let rendered = PrometheusEncoder::default().encode_window(&WindowMetrics {
            timestamp_unix_seconds: 2.0,
            protocol: "icmp".to_owned(),
            target: "example.com".to_owned(),
            duration_seconds: 10.0,
            samples: 4,
            replies: 3,
            lost: 1,
            loss_pct: 25.0,
            rtt_mean_seconds: Some(0.010),
            rtt_min_seconds: Some(0.005),
            rtt_max_seconds: Some(0.020),
        });

        assert!(rendered.contains("clockping_window_duration_seconds"));
        assert!(
            rendered
                .contains("clockping_window_lost{protocol=\"icmp\",target=\"example.com\"} 1\n")
        );
        assert!(rendered.contains("clockping_window_rtt_mean_seconds"));
    }

    #[test]
    fn invalid_metric_prefix_is_rejected() {
        assert!(PrometheusEncoder::new("bad-prefix").is_err());
    }

    #[test]
    fn invalid_labels_are_rejected() {
        for labels in [
            vec![("9bad", "value")],
            vec![("ok", "")],
            vec![("dup", "one"), ("dup", "two")],
        ] {
            assert!(PrometheusEncoder::with_labels("clockping", labels).is_err());
        }
    }
}
