use std::{future::pending, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use tokio::time;

use crate::{
    event::{ProbeEvent, ProbeOutcome, Recovery},
    metrics::{ProbeMetrics, SharedMetricsReporter},
    output::Output,
};

#[async_trait]
pub trait Prober {
    fn protocol(&self) -> &'static str;
    fn target(&self) -> &str;
    async fn probe(&mut self, seq: u64) -> ProbeOutcome;
}

#[derive(Clone, Copy, Debug)]
pub struct RunnerConfig {
    pub interval: Duration,
    pub count: Option<u64>,
    pub deadline: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct LossPeriod {
    pub start: DateTime<Local>,
    pub end: Option<DateTime<Local>>,
    pub lost: u64,
}

#[derive(Debug, Clone)]
struct OpenLossPeriod {
    start: DateTime<Local>,
    lost: u64,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub target: String,
    pub sent: u64,
    pub received: u64,
    rtt_min_max: Option<(Duration, Duration)>,
    rtt_total_secs: f64,
    // Detailed loss history intentionally remains unbounded.
    pub loss_periods: Vec<LossPeriod>,
    open_loss: Option<OpenLossPeriod>,
}

impl Summary {
    pub fn new(target: String) -> Self {
        Self {
            target,
            sent: 0,
            received: 0,
            rtt_min_max: None,
            rtt_total_secs: 0.0,
            loss_periods: Vec::new(),
            open_loss: None,
        }
    }

    pub fn record(&mut self, ts: DateTime<Local>, outcome: &ProbeOutcome) -> Option<Recovery> {
        self.sent += 1;

        match outcome {
            ProbeOutcome::Reply { rtt, .. } => {
                self.received += 1;
                let (min, max) = self.rtt_min_max.get_or_insert((*rtt, *rtt));
                *min = (*min).min(*rtt);
                *max = (*max).max(*rtt);
                self.rtt_total_secs += rtt.as_secs_f64();
                self.open_loss.take().map(|open| {
                    let duration_ms = ts
                        .signed_duration_since(open.start)
                        .num_milliseconds()
                        .max(0) as u128;
                    self.loss_periods.push(LossPeriod {
                        start: open.start,
                        end: Some(ts),
                        lost: open.lost,
                    });
                    Recovery {
                        lost: open.lost,
                        duration_ms,
                    }
                })
            }
            ProbeOutcome::Timeout { .. } | ProbeOutcome::Error(_) => {
                if let Some(open) = &mut self.open_loss {
                    open.lost += 1;
                } else {
                    self.open_loss = Some(OpenLossPeriod { start: ts, lost: 1 });
                }
                None
            }
        }
    }

    pub fn finalize(&mut self) {
        if let Some(open) = self.open_loss.take() {
            self.loss_periods.push(LossPeriod {
                start: open.start,
                end: None,
                lost: open.lost,
            });
        }
    }

    pub fn rtt_min_avg_max(&self) -> Option<(Duration, Duration, Duration)> {
        let (min, max) = self.rtt_min_max?;
        let avg = Duration::from_secs_f64(self.rtt_total_secs / self.received as f64);
        Some((min, avg, max))
    }
}

pub async fn run_probe_loop<P: Prober + Send>(
    mut prober: P,
    config: RunnerConfig,
    output: Output,
    quiet: bool,
    metrics: Option<SharedMetricsReporter>,
) -> Result<Summary> {
    let interval_duration = if config.interval.is_zero() {
        Duration::from_nanos(1)
    } else {
        config.interval
    };
    let mut interval = time::interval(interval_duration);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let deadline_at = config
        .deadline
        .map(|duration| time::Instant::now() + duration);
    let deadline = async {
        match deadline_at {
            Some(deadline) => time::sleep_until(deadline).await,
            None => pending::<()>().await,
        }
    };
    tokio::pin!(deadline);
    let mut seq = 0_u64;
    let mut summary = Summary::new(prober.target().to_string());

    loop {
        if config.count.is_some_and(|count| seq >= count) {
            break;
        }
        tokio::select! {
            biased;
            _ = &mut deadline => break,
            signal = tokio::signal::ctrl_c() => {
                signal?;
                break;
            }
            _ = interval.tick() => {}
        }
        if deadline_at.is_some_and(|deadline| time::Instant::now() >= deadline) {
            break;
        }

        let ts = Local::now();
        let outcome = tokio::select! {
            biased;
            _ = &mut deadline => break,
            signal = tokio::signal::ctrl_c() => {
                signal?;
                break;
            }
            outcome = prober.probe(seq) => outcome,
        };
        let recovery = summary.record(ts, &outcome);
        let event = ProbeEvent {
            ts,
            protocol: prober.protocol(),
            target: prober.target().to_string(),
            seq,
            outcome,
            recovery,
        };
        // Commit completed events before stdout errors or transport cancellation.
        let pending_push = if let Some(metrics) = &metrics {
            metrics
                .lock()
                .await
                .record(ProbeMetrics::from_event(&event, &summary))?
        } else {
            None
        };
        if !quiet {
            output.print_event(&event)?;
        }
        seq += 1;
        // FIFO saturation applies backpressure outside the shared aggregation lock.
        // Probe output and file events are already recorded; cancellation is visible.
        if let Some(pending) = pending_push {
            tokio::select! {
                biased;
                _ = &mut deadline => {
                    eprintln!("metrics enqueue reached probe deadline; pending snapshot was not queued");
                    break;
                }
                signal = tokio::signal::ctrl_c() => {
                    signal?;
                    eprintln!("metrics enqueue interrupted; pending snapshot was not queued");
                    break;
                }
                result = pending.send(metrics.as_ref().expect("pending push has reporter")) => result?,
            }
        }
    }

    summary.finalize();
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tokio::net::UdpSocket;

    use crate::{
        metrics::MetricsReporter,
        metrics_file::{MetricsFileFormat, MetricsFileSink},
        protocol::gtp::{GtpProber, GtpVariant},
        timefmt::{TimestampFormatter, TimestampKind},
    };

    use super::*;

    #[tokio::test]
    async fn deadline_bounds_ticks_and_active_probes() {
        for (respond, count, deadline, expected) in [
            (false, Some(1), Some(Duration::from_millis(50)), 0),
            (true, None, Some(Duration::from_millis(50)), 1),
            (true, Some(1), None, 1),
            (true, Some(2), None, 2),
            (true, Some(0), None, 0),
            (true, Some(1), Some(Duration::ZERO), 0),
        ] {
            let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let prober = GtpProber::new(
                GtpVariant::V1u,
                "127.0.0.1".to_owned(),
                Some(server.local_addr().unwrap().port()),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
            let started = time::Instant::now();
            let run = async {
                let summary = time::timeout(
                    Duration::from_millis(400),
                    run_probe_loop(
                        prober,
                        RunnerConfig {
                            interval: if deadline.is_some() {
                                Duration::from_secs(1)
                            } else {
                                Duration::from_millis(10)
                            },
                            count,
                            deadline,
                        },
                        Output::new(
                            TimestampFormatter::new(TimestampKind::None, None),
                            false,
                            false,
                        ),
                        true,
                        None,
                    ),
                )
                .await
                .expect("runner overran deadline/count")
                .unwrap();
                assert!(started.elapsed() < Duration::from_millis(400));
                if let Some(deadline) = deadline {
                    assert!(started.elapsed() >= deadline);
                }
                summary
            };
            let fixture = async {
                let mut requests = 0;
                let mut buf = [0; 64];
                while let Ok(Ok((len, peer))) =
                    time::timeout(Duration::from_millis(200), server.recv_from(&mut buf)).await
                {
                    assert_eq!(u16::from_be_bytes([buf[8], buf[9]]) as u64, requests);
                    requests += 1;
                    if respond {
                        buf[1] = 2;
                        server.send_to(&buf[..len], peer).await.unwrap();
                    }
                }
                requests
            };
            let (summary, requests) = tokio::join!(run, fixture);
            assert_eq!(summary.sent, expected);
            assert_eq!(summary.received, if respond { expected } else { 0 });
            assert_eq!(requests, if respond { expected } else { 1 });
        }
    }

    #[tokio::test]
    async fn deadline_preserves_completed_metrics_after_finite_lock_wait() {
        let path = std::env::temp_dir().join(format!(
            "clockping-deadline-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let prober = GtpProber::new(
            GtpVariant::V1u,
            "127.0.0.1".to_owned(),
            Some(server.local_addr().unwrap().port()),
            Duration::from_secs(1),
        )
        .await
        .unwrap();
        let sink = MetricsFileSink::with_prefix_and_labels(
            &path,
            MetricsFileFormat::Jsonl,
            "clockping",
            std::iter::empty::<(String, String)>(),
        )
        .unwrap();
        let metrics = MetricsReporter::new(None, Some(sink)).shared();
        let guard = metrics.lock().await;
        let run = run_probe_loop(
            prober,
            RunnerConfig {
                interval: Duration::from_millis(10),
                count: None,
                deadline: Some(Duration::from_millis(50)),
            },
            Output::new(
                TimestampFormatter::new(TimestampKind::None, None),
                false,
                false,
            ),
            true,
            Some(metrics.clone()),
        );
        let fixture = async {
            let mut buf = [0; 64];
            let (len, peer) = server.recv_from(&mut buf).await.unwrap();
            buf[1] = 2;
            server.send_to(&buf[..len], peer).await.unwrap();
        };
        let release = async {
            time::sleep(Duration::from_millis(80)).await;
            drop(guard);
        };
        let (summary, (), ()) = time::timeout(Duration::from_millis(400), async {
            tokio::join!(run, fixture, release)
        })
        .await
        .unwrap();
        let summary = summary.unwrap();
        assert_eq!((summary.sent, summary.received), (1, 1));
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 1);
        let event: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(event["status"], "reply");
        assert_eq!(event["sent"], 1);
        assert_eq!(
            server.try_recv(&mut [0; 64]).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn summary_aggregates_many_rtts_without_sample_retention() {
        let mut summary = Summary::new("target".to_string());
        let ts = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 0).unwrap();
        assert_eq!(summary.rtt_min_avg_max(), None);

        const SAMPLES: u64 = 100_000;
        for index in 0..SAMPLES {
            let rtt = Duration::from_nanos((index * 37 + SAMPLES / 2) % SAMPLES + 1);
            assert!(
                summary
                    .record(
                        ts,
                        &ProbeOutcome::Reply {
                            rtt,
                            peer: String::new(),
                            bytes: None,
                            ttl: None,
                            detail: Vec::new(),
                        }
                    )
                    .is_none()
            );
            if index == 0 {
                assert_eq!(summary.rtt_min_avg_max(), Some((rtt, rtt, rtt)));
            }
        }
        let total_secs = (0..SAMPLES)
            .map(|index| {
                Duration::from_nanos((index * 37 + SAMPLES / 2) % SAMPLES + 1).as_secs_f64()
            })
            .sum::<f64>();
        assert_eq!(
            summary.rtt_min_avg_max(),
            Some((
                Duration::from_nanos(1),
                Duration::from_secs_f64(total_secs / SAMPLES as f64),
                Duration::from_nanos(SAMPLES),
            ))
        );
        assert_eq!((summary.sent, summary.received), (SAMPLES, SAMPLES));
        // This Copy-only state cannot retain heap-allocated RTT samples.
        let rtt_state: (Option<(Duration, Duration)>, f64) =
            (summary.rtt_min_max, summary.rtt_total_secs);
        let copy = rtt_state;
        assert_eq!(rtt_state, copy);
        assert_eq!(summary.loss_periods.capacity(), 0);
    }

    #[test]
    fn summary_tracks_loss_period_recovery() {
        let mut summary = Summary::new("target".to_string());
        let t0 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 0).unwrap();
        let t1 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 1).unwrap();
        let t2 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 2).unwrap();

        assert!(
            summary
                .record(t0, &ProbeOutcome::Timeout { detail: Vec::new() })
                .is_none()
        );
        assert_eq!(summary.rtt_min_avg_max(), None);
        assert!(
            summary
                .record(t1, &ProbeOutcome::Error("failed".to_string()))
                .is_none()
        );
        let recovery = summary
            .record(
                t2,
                &ProbeOutcome::Reply {
                    rtt: Duration::from_millis(10),
                    peer: "127.0.0.1".to_string(),
                    bytes: Some(64),
                    ttl: Some(64),
                    detail: Vec::new(),
                },
            )
            .unwrap();

        assert_eq!(recovery.lost, 2);
        assert_eq!(recovery.duration_ms, 2000);
        assert_eq!(summary.loss_periods.len(), 1);
        assert_eq!(summary.loss_periods[0].lost, 2);
        assert_eq!(summary.loss_periods[0].start, t0);
        assert_eq!(summary.loss_periods[0].end, Some(t2));
        assert_eq!(summary.sent, 3);
        assert_eq!(summary.received, 1);
        let rtt = Duration::from_millis(10);
        assert_eq!(summary.rtt_min_avg_max(), Some((rtt, rtt, rtt)));

        summary.record(t2, &ProbeOutcome::Timeout { detail: Vec::new() });
        summary.finalize();
        summary.finalize();
        assert_eq!((summary.sent, summary.received), (4, 1));
        assert_eq!(summary.rtt_min_avg_max(), Some((rtt, rtt, rtt)));
        assert_eq!(summary.loss_periods.len(), 2);
        assert_eq!(summary.loss_periods[1].lost, 1);
        assert_eq!(summary.loss_periods[1].start, t2);
        assert_eq!(summary.loss_periods[1].end, None);
    }
}
