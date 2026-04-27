use std::{
    net::{IpAddr, SocketAddr},
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

use async_trait::async_trait;
use surge_ping::{Client, Config, ICMP, IcmpPacket, PingIdentifier, PingSequence};
use tokio::net::lookup_host;

use crate::{event::ProbeOutcome, runner::Prober};

static NEXT_PING_IDENTIFIER_OFFSET: AtomicU16 = AtomicU16::new(0);

#[derive(Debug, Clone)]
pub struct NativeIcmpConfig {
    pub destinations: Vec<String>,
    pub ipv4: bool,
    pub ipv6: bool,
    pub count: Option<u64>,
    pub interval: Duration,
    pub timeout: Duration,
    pub deadline: Option<Duration>,
    pub size: usize,
    pub ttl: Option<u32>,
    pub interface_or_source: Option<String>,
    pub numeric: bool,
    pub quiet: bool,
    pub timestamp: bool,
    pub report_outstanding: bool,
}

pub struct NativeIcmpProber {
    target: String,
    _client: Client,
    pinger: surge_ping::Pinger,
    payload: Vec<u8>,
    report_outstanding: bool,
}

impl NativeIcmpProber {
    pub async fn new(config: NativeIcmpConfig) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !(config.ipv4 && config.ipv6),
            "-4 and -6 cannot be used together"
        );
        let destination = config
            .destinations
            .first()
            .ok_or_else(|| anyhow::anyhow!("missing ICMP destination"))?;
        let host = resolve_icmp_host(destination, config.ipv4, config.ipv6).await?;
        let mut builder = Config::builder();
        if host.is_ipv6() {
            builder = builder.kind(ICMP::V6);
        }
        if let Some(ttl) = config.ttl {
            builder = builder.ttl(ttl);
        }
        if let Some(interface_or_source) = &config.interface_or_source {
            if let Ok(source) = interface_or_source.parse::<IpAddr>() {
                builder = builder.bind(SocketAddr::new(source, 0));
            } else {
                builder = builder.interface(interface_or_source);
            }
        }

        let client = Client::new(&builder.build())?;
        let ident = next_ping_identifier();
        let mut pinger = client.pinger(host, ident).await;
        pinger.timeout(config.timeout);

        Ok(Self {
            target: if config.numeric {
                host.to_string()
            } else {
                format!("{destination} ({host})")
            },
            _client: client,
            pinger,
            payload: vec![0; config.size],
            report_outstanding: config.report_outstanding,
        })
    }
}

fn next_ping_identifier() -> PingIdentifier {
    let process_id = (std::process::id() & 0xffff) as u16;
    let offset = NEXT_PING_IDENTIFIER_OFFSET.fetch_add(1, Ordering::Relaxed);
    PingIdentifier(process_id.wrapping_add(offset))
}

async fn resolve_icmp_host(destination: &str, ipv4: bool, ipv6: bool) -> anyhow::Result<IpAddr> {
    if let Ok(ip) = destination.parse::<IpAddr>() {
        anyhow::ensure!(
            !ipv4 || ip.is_ipv4(),
            "{destination} is not an IPv4 address"
        );
        anyhow::ensure!(
            !ipv6 || ip.is_ipv6(),
            "{destination} is not an IPv6 address"
        );
        return Ok(ip);
    }

    let addresses = lookup_host((destination, 0)).await?.collect::<Vec<_>>();
    addresses
        .into_iter()
        .map(|addr| addr.ip())
        .find(|ip| (!ipv4 || ip.is_ipv4()) && (!ipv6 || ip.is_ipv6()))
        .ok_or_else(|| anyhow::anyhow!("no matching addresses resolved for {destination}"))
}

#[async_trait]
impl Prober for NativeIcmpProber {
    fn protocol(&self) -> &'static str {
        "icmp"
    }

    fn target(&self) -> &str {
        &self.target
    }

    async fn probe(&mut self, seq: u64) -> ProbeOutcome {
        let ping_seq = PingSequence((seq & 0xffff) as u16);
        match self.pinger.ping(ping_seq, &self.payload).await {
            Ok((IcmpPacket::V4(packet), rtt)) => ProbeOutcome::Reply {
                rtt,
                peer: packet.get_source().to_string(),
                bytes: Some(packet.get_size()),
                ttl: packet.get_ttl(),
                detail: vec![("icmp_seq".to_string(), packet.get_sequence().0.to_string())],
            },
            Ok((IcmpPacket::V6(packet), rtt)) => ProbeOutcome::Reply {
                rtt,
                peer: packet.get_source().to_string(),
                bytes: Some(packet.get_size()),
                ttl: Some(packet.get_max_hop_limit()),
                detail: vec![("icmp_seq".to_string(), packet.get_sequence().0.to_string())],
            },
            Err(surge_ping::SurgeError::Timeout { .. }) => {
                let detail = if self.report_outstanding {
                    vec![
                        ("icmp_seq".to_string(), ping_seq.0.to_string()),
                        ("outstanding".to_string(), "true".to_string()),
                    ]
                } else {
                    Vec::new()
                };
                ProbeOutcome::Timeout { detail }
            }
            Err(error) => ProbeOutcome::Error(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_identifiers_are_unique_per_native_prober() {
        assert_ne!(next_ping_identifier().0, next_ping_identifier().0);
    }
}
