use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};

use crate::{
    event::{ProbeEvent, ProbeOutcome},
    metrics_file::MetricsFileSink,
    pushgateway::PushGateway,
    runner::Summary,
};

pub type SharedMetricsReporter = Arc<Mutex<MetricsReporter>>;

const PUSH_QUEUE_CAPACITY: usize = 16;
const PUSH_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

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

    pub fn shared(mut self) -> SharedMetricsReporter {
        if let Some(pushgateway) = &mut self.pushgateway {
            pushgateway.start();
        }
        Arc::new(Mutex::new(self))
    }

    pub fn is_empty(&self) -> bool {
        self.pushgateway.is_none() && self.file.is_none()
    }

    pub fn record(&mut self, metrics: ProbeMetrics) -> anyhow::Result<Option<PendingPush>> {
        let key = MetricsKey::from_probe(&metrics);
        self.latest_intervals.insert(key, metrics.clone());

        if let Some(file) = &self.file {
            if file.writes_prometheus_snapshot() {
                file.write_intervals(&self.latest_intervals.values().cloned().collect::<Vec<_>>())?;
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
            Ok(self.record_window(metrics))
        } else if let Some(pushgateway) = &self.pushgateway {
            Ok(Some(pushgateway.pending(false)))
        } else {
            Ok(None)
        }
    }

    pub async fn finish(&mut self) {
        let deadline = tokio::time::Instant::now() + PUSH_SHUTDOWN_TIMEOUT;
        if self
            .pushgateway
            .as_ref()
            .and_then(|pushgateway| pushgateway.interval)
            .is_some()
        {
            let keys = self.windows.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if let Some(pending) = self.flush_window(&key) {
                    match tokio::time::timeout_at(deadline, pending.sender.reserve()).await {
                        Ok(Ok(permit)) => permit.send(self.snapshot(true)),
                        Ok(Err(error)) => {
                            eprintln!("failed to queue final window metrics: {error:#}")
                        }
                        Err(_) => eprintln!(
                            "final window metrics were not queued before shutdown timeout"
                        ),
                    }
                }
            }
        }
        if let Some(pushgateway) = &mut self.pushgateway {
            pushgateway.finish_transport(deadline).await;
            pushgateway.delete_on_finish().await;
        }
    }

    fn record_window(&mut self, metrics: ProbeMetrics) -> Option<PendingPush> {
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
        let pending = flush_key.and_then(|key| self.flush_window(&key));

        self.windows
            .entry(key)
            .or_insert_with(|| WindowState {
                started: now,
                samples: Vec::new(),
            })
            .samples
            .push(metrics);
        pending
    }

    fn flush_window(&mut self, key: &MetricsKey) -> Option<PendingPush> {
        let window = self.windows.remove(key)?;
        let metrics = aggregate_window(&window.samples)?;
        self.latest_windows.insert(key.clone(), metrics);
        self.pushgateway
            .as_ref()
            .map(|pushgateway| pushgateway.pending(true))
    }

    fn snapshot(&self, window: bool) -> PushBatch {
        if window {
            PushBatch::Windows(self.latest_windows.values().cloned().collect())
        } else {
            PushBatch::Intervals(self.latest_intervals.values().cloned().collect())
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
    sender: Option<mpsc::Sender<PushBatch>>,
    task: Option<JoinHandle<()>>,
}

#[derive(Debug)]
enum PushBatch {
    Intervals(Vec<ProbeMetrics>),
    Windows(Vec<WindowMetrics>),
}

pub struct PendingPush {
    sender: mpsc::Sender<PushBatch>,
    window: bool,
}

impl PendingPush {
    pub async fn send(self, reporter: &SharedMetricsReporter) -> anyhow::Result<()> {
        let permit = self
            .sender
            .reserve()
            .await
            .map_err(|_| anyhow::anyhow!("metrics transport task stopped"))?;
        // Build and enqueue under the same lock, so concurrent targets cannot send
        // a stale, single-target snapshot after a newer multi-target snapshot.
        let reporter = reporter.lock().await;
        permit.send(reporter.snapshot(self.window));
        Ok(())
    }
}

impl PushGatewaySink {
    pub fn new(sink: PushGateway, interval: Option<Duration>) -> Self {
        Self {
            sink,
            interval,
            sender: None,
            task: None,
        }
    }

    fn start(&mut self) {
        let (sender, mut receiver) = mpsc::channel(PUSH_QUEUE_CAPACITY);
        let sink = self.sink.clone();
        self.sender = Some(sender);
        self.task = Some(tokio::spawn(async move {
            while let Some(batch) = receiver.recv().await {
                let (result, kind) = match batch {
                    PushBatch::Intervals(metrics) => (sink.push_many(&metrics).await, "metrics"),
                    PushBatch::Windows(metrics) => {
                        (sink.push_windows(&metrics).await, "window metrics")
                    }
                };
                if let Err(error) = result {
                    eprintln!("failed to push {kind}: {error:#}");
                }
            }
        }));
    }

    fn pending(&self, window: bool) -> PendingPush {
        PendingPush {
            sender: self
                .sender
                .as_ref()
                .expect("shared reporter starts transport")
                .clone(),
            window,
        }
    }

    async fn finish_transport(&mut self, deadline: tokio::time::Instant) {
        self.sender.take();
        if let Some(mut task) = self.task.take() {
            match tokio::time::timeout_at(deadline, &mut task).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("metrics transport task failed: {error}"),
                Err(_) => {
                    eprintln!(
                        "metrics transport shutdown timed out; unfinished pushes were canceled"
                    );
                    task.abort();
                    let _ = task.await;
                }
            }
        }
    }

    async fn delete_on_finish(&self) {
        if !self.sink.delete_on_finish() {
            return;
        }
        match tokio::time::timeout(PUSH_SHUTDOWN_TIMEOUT, self.sink.delete()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => eprintln!("failed to delete Pushgateway metrics: {error:#}"),
            Err(_) => eprintln!("Pushgateway delete timed out during shutdown"),
        }
    }
}

impl Drop for PushGatewaySink {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
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

    #[tokio::test]
    async fn queued_snapshots_cannot_regress_target_identity_and_queue_is_bounded() {
        use crate::pushgateway::PushGatewayConfig;

        let gateway = PushGateway::new(PushGatewayConfig {
            endpoint: reqwest::Url::parse("http://127.0.0.1:1").unwrap(),
            job: "test".to_owned(),
            labels: Vec::new(),
            timeout: Duration::from_secs(1),
            retries: 0,
            user_agent: "clockping/test".to_owned(),
            metric_prefix: "clockping".to_owned(),
            delete_on_finish: false,
        })
        .unwrap();
        let (sender, mut receiver) = mpsc::channel(PUSH_QUEUE_CAPACITY);
        let mut sink = PushGatewaySink::new(gateway, None);
        sink.sender = Some(sender.clone());
        let reporter = Arc::new(Mutex::new(MetricsReporter::new(Some(sink), None)));
        let sample = |target: &str| ProbeMetrics {
            timestamp_unix_seconds: 1.0,
            protocol: "tcp".to_owned(),
            target: target.to_owned(),
            seq: 0,
            status: "reply",
            sent: 1,
            received: 1,
            lost: 0,
            loss_pct: 0.0,
            up: 1.0,
            rtt_seconds: Some(0.001),
            bytes: None,
            ttl: None,
        };
        let first = reporter
            .lock()
            .await
            .record(sample("one:443"))
            .unwrap()
            .unwrap();
        let second = reporter
            .lock()
            .await
            .record(sample("two:443"))
            .unwrap()
            .unwrap();
        second.send(&reporter).await.unwrap();
        first.send(&reporter).await.unwrap();
        for _ in 0..2 {
            let PushBatch::Intervals(snapshot) = receiver.recv().await.unwrap() else {
                panic!("wrong snapshot kind")
            };
            assert_eq!(snapshot.len(), 2);
            assert_eq!(snapshot[0].target, "one:443");
            assert_eq!(snapshot[1].target, "two:443");
        }
        for _ in 0..PUSH_QUEUE_CAPACITY {
            sender.try_send(PushBatch::Intervals(Vec::new())).unwrap();
        }
        assert_eq!(sender.capacity(), 0);
        let pending = reporter
            .lock()
            .await
            .record(sample("one:443"))
            .unwrap()
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), pending.send(&reporter))
                .await
                .is_err()
        );
        // A backpressured producer does not own the aggregation lock.
        assert!(reporter.try_lock().is_ok());
    }

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
