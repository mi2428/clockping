mod labels;
mod render;

use crate::metrics::{ProbeMetrics, WindowMetrics};

pub(crate) use labels::{is_valid_label_name, reject_duplicate_labels, validate_metric_prefix};
pub(crate) use render::{
    render_interval_prometheus, render_interval_prometheus_many, render_window_prometheus,
    render_window_prometheus_many,
};

use self::{
    labels::validate_prometheus_labels,
    render::{
        render_interval_prometheus_many_with_labels, render_interval_prometheus_with_labels,
        render_window_prometheus_with_labels,
    },
};

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
