use crate::metrics::{ProbeMetrics, WindowMetrics};

pub(crate) fn render_interval_prometheus(metrics: &ProbeMetrics, prefix: &str) -> String {
    render_interval_prometheus_with_labels(metrics, prefix, &[])
}

pub(crate) fn render_interval_prometheus_many(metrics: &[ProbeMetrics], prefix: &str) -> String {
    render_interval_prometheus_many_with_labels(metrics, prefix, &[])
}

pub(crate) fn render_window_prometheus(metrics: &WindowMetrics, prefix: &str) -> String {
    render_window_prometheus_with_labels(metrics, prefix, &[])
}

pub(crate) fn render_window_prometheus_many(metrics: &[WindowMetrics], prefix: &str) -> String {
    render_window_prometheus_many_with_labels(metrics, prefix, &[])
}

pub(super) fn render_interval_prometheus_with_labels(
    metrics: &ProbeMetrics,
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    render_interval_prometheus_many_with_labels(std::slice::from_ref(metrics), prefix, labels)
}

pub(super) fn render_interval_prometheus_many_with_labels(
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

pub(super) fn render_window_prometheus_with_labels(
    metrics: &WindowMetrics,
    prefix: &str,
    labels: &[(String, String)],
) -> String {
    render_window_prometheus_many_with_labels(std::slice::from_ref(metrics), prefix, labels)
}

pub(super) fn render_window_prometheus_many_with_labels(
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
