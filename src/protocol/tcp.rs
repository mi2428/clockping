use std::{net::SocketAddr, time::Duration};

use anyhow::Context;
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
        let target = normalize_tcp_target(&target)?;
        let resolved = lookup_host(target.as_str())
            .await
            .with_context(|| format!("failed to resolve TCP target {target}"))?
            .collect::<Vec<_>>();
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

pub fn normalize_tcp_target(target: &str) -> anyhow::Result<String> {
    let target = target.trim();
    anyhow::ensure!(!target.is_empty(), "TCP target must not be empty");
    if target.parse::<SocketAddr>().is_ok() {
        return Ok(target.to_string());
    }
    if let Some((host, port)) = target.rsplit_once(':') {
        anyhow::ensure!(
            !host.is_empty() && !host.contains(':') && !port.is_empty(),
            "invalid TCP target {target:?}; use host:port or [ipv6]:port"
        );
        let port = port
            .parse::<u16>()
            .with_context(|| format!("invalid TCP port in {target:?}; expected 0-65535"))?;
        return Ok(format!("{host}:{port}"));
    }
    anyhow::bail!("TCP target must include a port; use host:port, for example {target}:443")
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

    #[test]
    fn tcp_target_keeps_explicit_port() {
        assert_eq!(
            normalize_tcp_target("example.com:443").unwrap(),
            "example.com:443"
        );
        assert_eq!(normalize_tcp_target("[::1]:443").unwrap(), "[::1]:443");
    }

    #[test]
    fn tcp_target_rejects_invalid_port() {
        assert!(normalize_tcp_target("example.com:https").is_err());
    }

    #[test]
    fn tcp_target_rejects_missing_port() {
        let error = normalize_tcp_target("example.com").unwrap_err().to_string();

        assert!(error.contains("TCP target must include a port"));
    }
}
