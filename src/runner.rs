use std::time::{Duration, Instant};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use tokio::time;

use crate::{
    event::{ProbeEvent, ProbeOutcome, Recovery},
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
    pub rtts: Vec<Duration>,
    pub loss_periods: Vec<LossPeriod>,
    open_loss: Option<OpenLossPeriod>,
}

impl Summary {
    pub fn new(target: String) -> Self {
        Self {
            target,
            sent: 0,
            received: 0,
            rtts: Vec::new(),
            loss_periods: Vec::new(),
            open_loss: None,
        }
    }

    pub fn record(&mut self, ts: DateTime<Local>, outcome: &ProbeOutcome) -> Option<Recovery> {
        self.sent += 1;

        match outcome {
            ProbeOutcome::Reply { rtt, .. } => {
                self.received += 1;
                self.rtts.push(*rtt);
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
            ProbeOutcome::Timeout | ProbeOutcome::Error(_) => {
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
        let min = *self.rtts.iter().min()?;
        let max = *self.rtts.iter().max()?;
        let total_secs = self.rtts.iter().map(Duration::as_secs_f64).sum::<f64>();
        let avg = Duration::from_secs_f64(total_secs / self.rtts.len() as f64);
        Some((min, avg, max))
    }
}

pub async fn run_probe_loop<P: Prober + Send>(
    mut prober: P,
    config: RunnerConfig,
    output: Output,
    quiet: bool,
) -> Result<Summary> {
    let interval_duration = if config.interval.is_zero() {
        Duration::from_nanos(1)
    } else {
        config.interval
    };
    let mut interval = time::interval(interval_duration);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

    let started = Instant::now();
    let mut seq = 0_u64;
    let mut summary = Summary::new(prober.target().to_string());

    loop {
        if config.count.is_some_and(|count| seq >= count) {
            break;
        }
        if config
            .deadline
            .is_some_and(|deadline| started.elapsed() >= deadline)
        {
            break;
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                break;
            }
            _ = interval.tick() => {}
        }

        let ts = Local::now();
        let outcome = prober.probe(seq).await;
        let recovery = summary.record(ts, &outcome);
        if !quiet {
            output.print_event(&ProbeEvent {
                ts,
                protocol: prober.protocol(),
                target: prober.target().to_string(),
                seq,
                outcome,
                recovery,
            })?;
        }
        seq += 1;
    }

    summary.finalize();
    output.print_summary(&summary, quiet)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn summary_tracks_loss_period_recovery() {
        let mut summary = Summary::new("target".to_string());
        let t0 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 0).unwrap();
        let t1 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 1).unwrap();
        let t2 = Local.with_ymd_and_hms(2026, 4, 25, 12, 0, 2).unwrap();

        assert!(summary.record(t0, &ProbeOutcome::Timeout).is_none());
        assert!(summary.record(t1, &ProbeOutcome::Timeout).is_none());
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
        assert_eq!(summary.loss_periods.len(), 1);
        assert_eq!(summary.loss_periods[0].lost, 2);
        assert_eq!(summary.sent, 3);
        assert_eq!(summary.received, 1);
    }
}
