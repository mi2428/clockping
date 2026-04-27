use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::sync::Mutex;

use crate::{
    event::{ProbeEvent, ProbeOutcome},
    metrics_file::MetricsFileSink,
    pushgateway::PushGateway,
    runner::Summary,
};

pub type SharedMetricsReporter = Arc<Mutex<MetricsReporter>>;

#[derive(Debug, Clone, Serialize)]
pub struct ProbeMetrics {
    pub timestamp_unix_seconds: f64,
    pub protocol: String,
    pub target: String,
    pub seq: u64,
    pub status: &'static str,
    pub sent: u64,
    pub received: u64,
    pub lost: u64,
    pub loss_pct: f64,
    pub up: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u8>,
}

impl ProbeMetrics {
    pub fn from_event(event: &ProbeEvent, summary: &Summary) -> Self {
        let lost = summary.sent.saturating_sub(summary.received);
        let loss_pct = if summary.sent == 0 {
            0.0
        } else {
            lost as f64 / summary.sent as f64 * 100.0
        };
        let (status, up, rtt_seconds, bytes, ttl) = match &event.outcome {
            ProbeOutcome::Reply {
                rtt, bytes, ttl, ..
            } => (
                "reply",
                1.0,
                Some(rtt.as_secs_f64()),
                bytes.map(|value| value as u64),
                *ttl,
            ),
            ProbeOutcome::Timeout { .. } => ("timeout", 0.0, None, None, None),
            ProbeOutcome::Error(_) => ("error", 0.0, None, None, None),
        };

        Self {
            timestamp_unix_seconds: current_unix_timestamp_seconds(),
            protocol: event.protocol.to_owned(),
            target: event.target.clone(),
            seq: event.seq,
            status,
            sent: summary.sent,
            received: summary.received,
            lost,
            loss_pct,
            up,
            rtt_seconds,
            bytes,
            ttl,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct WindowMetrics {
    pub timestamp_unix_seconds: f64,
    pub protocol: String,
    pub target: String,
    pub duration_seconds: f64,
    pub samples: u64,
    pub replies: u64,
    pub lost: u64,
    pub loss_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_mean_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_min_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_max_seconds: Option<f64>,
}

#[derive(Debug)]
pub struct MetricsReporter {
    pushgateway: Option<PushGatewaySink>,
    file: Option<MetricsFileSink>,
    latest_intervals: BTreeMap<MetricsKey, ProbeMetrics>,
    latest_windows: BTreeMap<MetricsKey, WindowMetrics>,
    windows: BTreeMap<MetricsKey, WindowState>,
}

impl MetricsReporter {
    pub fn new(pushgateway: Option<PushGatewaySink>, file: Option<MetricsFileSink>) -> Self {
        Self {
            pushgateway,
            file,
            latest_intervals: BTreeMap::new(),
            latest_windows: BTreeMap::new(),
            windows: BTreeMap::new(),
        }
    }

    pub fn shared(self) -> SharedMetricsReporter {
        Arc::new(Mutex::new(self))
    }

    pub fn is_empty(&self) -> bool {
        self.pushgateway.is_none() && self.file.is_none()
    }

    pub async fn record(&mut self, metrics: ProbeMetrics) -> anyhow::Result<()> {
        let key = MetricsKey::from_probe(&metrics);
        self.latest_intervals.insert(key, metrics.clone());
        let interval_snapshot = self.latest_intervals.values().cloned().collect::<Vec<_>>();

        if let Some(file) = &self.file {
            if file.writes_prometheus_snapshot() {
                file.write_intervals(&interval_snapshot)?;
            } else {
                file.write_interval(&metrics)?;
            }
        }

        if self
            .pushgateway
            .as_ref()
            .and_then(|pushgateway| pushgateway.interval)
            .is_some()
        {
            self.record_window(metrics).await;
        } else if let Some(pushgateway) = &self.pushgateway {
            pushgateway.push_intervals(&interval_snapshot).await;
        }

        Ok(())
    }

    pub async fn finish(&mut self) {
        if self
            .pushgateway
            .as_ref()
            .and_then(|pushgateway| pushgateway.interval)
            .is_some()
        {
            self.flush_all_windows().await;
        }
        if let Some(pushgateway) = &self.pushgateway {
            pushgateway.delete_on_finish().await;
        }
    }

    async fn record_window(&mut self, metrics: ProbeMetrics) {
        let now = Instant::now();
        let key = MetricsKey::from_probe(&metrics);
        let mut flush_key = None;
        if let Some(window) = self.windows.get(&key)
            && let Some(pushgateway) = &self.pushgateway
            && let Some(interval) = pushgateway.interval
            && now.duration_since(window.started) >= interval
        {
            flush_key = Some(key.clone());
        }
        if let Some(key) = flush_key {
            self.flush_window(&key).await;
        }

        self.windows
            .entry(key)
            .or_insert_with(|| WindowState {
                started: now,
                samples: Vec::new(),
            })
            .samples
            .push(metrics);
    }

    async fn flush_window(&mut self, key: &MetricsKey) {
        let Some(window) = self.windows.remove(key) else {
            return;
        };
        let Some(metrics) = aggregate_window(&window.samples) else {
            return;
        };
        self.latest_windows.insert(key.clone(), metrics);
        if let Some(pushgateway) = &self.pushgateway {
            let snapshot = self.latest_windows.values().cloned().collect::<Vec<_>>();
            pushgateway.push_windows(&snapshot).await;
        }
    }

    async fn flush_all_windows(&mut self) {
        let keys = self.windows.keys().cloned().collect::<Vec<_>>();
        for key in keys {
            self.flush_window(&key).await;
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MetricsKey {
    protocol: String,
    target: String,
}

impl MetricsKey {
    fn from_probe(metrics: &ProbeMetrics) -> Self {
        Self {
            protocol: metrics.protocol.clone(),
            target: metrics.target.clone(),
        }
    }
}

#[derive(Debug)]
struct WindowState {
    started: Instant,
    samples: Vec<ProbeMetrics>,
}

#[derive(Debug)]
pub struct PushGatewaySink {
    sink: PushGateway,
    interval: Option<Duration>,
}

impl PushGatewaySink {
    pub fn new(sink: PushGateway, interval: Option<Duration>) -> Self {
        Self { sink, interval }
    }

    async fn push_intervals(&self, metrics: &[ProbeMetrics]) {
        if let Err(error) = self.sink.push_many(metrics).await {
            eprintln!("failed to push metrics: {error:#}");
        }
    }

    async fn push_windows(&self, metrics: &[WindowMetrics]) {
        if let Err(error) = self.sink.push_windows(metrics).await {
            eprintln!("failed to push window metrics: {error:#}");
        }
    }

    async fn delete_on_finish(&self) {
        if !self.sink.delete_on_finish() {
            return;
        }
        if let Err(error) = self.sink.delete().await {
            eprintln!("failed to delete Pushgateway metrics: {error:#}");
        }
    }
}

pub fn aggregate_window(samples: &[ProbeMetrics]) -> Option<WindowMetrics> {
    let first = samples.first()?;
    let last = samples.last().unwrap_or(first);
    let mut rtts = samples
        .iter()
        .filter_map(|sample| sample.rtt_seconds)
        .collect::<Vec<_>>();
    rtts.sort_by(f64::total_cmp);
    let replies = rtts.len() as u64;
    let samples_len = samples.len() as u64;
    let lost = samples_len.saturating_sub(replies);
    let loss_pct = if samples_len == 0 {
        0.0
    } else {
        lost as f64 / samples_len as f64 * 100.0
    };
    let rtt_mean_seconds = (!rtts.is_empty()).then(|| rtts.iter().sum::<f64>() / rtts.len() as f64);
    let duration_seconds = (last.timestamp_unix_seconds - first.timestamp_unix_seconds).max(0.0);

    Some(WindowMetrics {
        timestamp_unix_seconds: last.timestamp_unix_seconds,
        protocol: first.protocol.clone(),
        target: first.target.clone(),
        duration_seconds,
        samples: samples_len,
        replies,
        lost,
        loss_pct,
        rtt_mean_seconds,
        rtt_min_seconds: rtts.first().copied(),
        rtt_max_seconds: rtts.last().copied(),
    })
}

fn current_unix_timestamp_seconds() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_metrics_aggregate_probe_results() {
        let window = aggregate_window(&[
            ProbeMetrics {
                timestamp_unix_seconds: 1.0,
                protocol: "tcp".to_owned(),
                target: "example:443".to_owned(),
                seq: 0,
                status: "reply",
                sent: 1,
                received: 1,
                lost: 0,
                loss_pct: 0.0,
                up: 1.0,
                rtt_seconds: Some(0.010),
                bytes: None,
                ttl: None,
            },
            ProbeMetrics {
                timestamp_unix_seconds: 2.5,
                protocol: "tcp".to_owned(),
                target: "example:443".to_owned(),
                seq: 1,
                status: "timeout",
                sent: 2,
                received: 1,
                lost: 1,
                loss_pct: 50.0,
                up: 0.0,
                rtt_seconds: None,
                bytes: None,
                ttl: None,
            },
        ])
        .unwrap();

        assert_eq!(window.protocol, "tcp");
        assert_eq!(window.samples, 2);
        assert_eq!(window.replies, 1);
        assert_eq!(window.lost, 1);
        assert_eq!(window.loss_pct, 50.0);
        assert_eq!(window.rtt_mean_seconds, Some(0.010));
        assert_eq!(window.duration_seconds, 1.5);
    }
}
