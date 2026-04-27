use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use tokio::{
    net::{TcpStream, lookup_host},
    time::Instant,
};

use crate::{event::ProbeOutcome, runner::Prober};

pub struct TcpProber {
    target: String,
    resolved: Vec<SocketAddr>,
    timeout: Duration,
    next_addr: usize,
}

impl TcpProber {
    pub async fn new(target: String, timeout: Duration) -> anyhow::Result<Self> {
        let resolved = lookup_host(&target).await?.collect::<Vec<_>>();
        anyhow::ensure!(!resolved.is_empty(), "no addresses resolved for {target}");
        Ok(Self {
            target,
            resolved,
            timeout,
            next_addr: 0,
        })
    }

    fn next_target(&mut self) -> SocketAddr {
        let addr = self.resolved[self.next_addr % self.resolved.len()];
        self.next_addr = self.next_addr.wrapping_add(1);
        addr
    }
}

#[async_trait]
impl Prober for TcpProber {
    fn protocol(&self) -> &'static str {
        "tcp"
    }

    fn target(&self) -> &str {
        &self.target
    }

    async fn probe(&mut self, _seq: u64) -> ProbeOutcome {
        let addr = self.next_target();
        let started = Instant::now();
        match tokio::time::timeout(self.timeout, TcpStream::connect(addr)).await {
            Ok(Ok(stream)) => {
                let rtt = started.elapsed();
                drop(stream);
                ProbeOutcome::Reply {
                    rtt,
                    peer: addr.to_string(),
                    bytes: None,
                    ttl: None,
                    detail: Vec::new(),
                }
            }
            Ok(Err(error)) => ProbeOutcome::Error(error.to_string()),
            Err(_) => ProbeOutcome::Timeout { detail: Vec::new() },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn tcp_probe_replies_to_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accept_task = tokio::spawn(async move {
            let _ = listener.accept().await;
        });

        let mut prober = TcpProber::new(addr.to_string(), Duration::from_secs(1))
            .await
            .unwrap();
        let outcome = prober.probe(0).await;
        assert!(matches!(outcome, ProbeOutcome::Reply { .. }));
        accept_task.await.unwrap();
    }
}
