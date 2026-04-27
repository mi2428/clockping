use std::{net::SocketAddr, time::Duration};

use async_trait::async_trait;
use tokio::{
    net::{UdpSocket, lookup_host},
    time::Instant,
};

use crate::{event::ProbeOutcome, runner::Prober};

pub mod codec;

use codec::{GtpCodec, GtpEchoReply};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GtpVariant {
    V1u,
    V1c,
    V2c,
}

impl GtpVariant {
    pub fn default_port(self) -> u16 {
        match self {
            Self::V1u => 2152,
            Self::V1c | Self::V2c => 2123,
        }
    }

    pub fn protocol_name(self) -> &'static str {
        match self {
            Self::V1u => "gtpv1u",
            Self::V1c => "gtpv1c",
            Self::V2c => "gtpv2c",
        }
    }

    fn codec(self) -> GtpCodec {
        match self {
            Self::V1u | Self::V1c => GtpCodec::V1,
            Self::V2c => GtpCodec::V2,
        }
    }
}

pub struct GtpProber {
    variant: GtpVariant,
    target: String,
    remote: SocketAddr,
    socket: UdpSocket,
    timeout: Duration,
}

impl GtpProber {
    pub async fn new(
        variant: GtpVariant,
        target: String,
        port: Option<u16>,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let port = port.unwrap_or_else(|| variant.default_port());
        let remote = lookup_host((target.as_str(), port))
            .await?
            .next()
            .ok_or_else(|| anyhow::anyhow!("no addresses resolved for {target}:{port}"))?;
        let bind_addr = if remote.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self {
            variant,
            target: format!("{target}:{port}"),
            remote,
            socket,
            timeout,
        })
    }

    async fn recv_matching_reply(
        &self,
        expected_sequence: u32,
        deadline: Instant,
    ) -> Result<(GtpEchoReply, SocketAddr), ProbeOutcome> {
        let mut buf = [0_u8; 2048];
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(ProbeOutcome::Timeout { detail: Vec::new() });
            }
            let remaining = deadline - now;
            let recv = tokio::time::timeout(remaining, self.socket.recv_from(&mut buf)).await;
            let (len, peer) = match recv {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => return Err(ProbeOutcome::Error(error.to_string())),
                Err(_) => return Err(ProbeOutcome::Timeout { detail: Vec::new() }),
            };

            match self.variant.codec().decode_echo_reply(&buf[..len]) {
                Ok(reply) if reply.sequence == expected_sequence => return Ok((reply, peer)),
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
    }
}

#[async_trait]
impl Prober for GtpProber {
    fn protocol(&self) -> &'static str {
        self.variant.protocol_name()
    }

    fn target(&self) -> &str {
        &self.target
    }

    async fn probe(&mut self, seq: u64) -> ProbeOutcome {
        let codec = self.variant.codec();
        let sequence = codec.sequence_from_u64(seq);
        let request = codec.encode_echo_request(sequence);
        let started = Instant::now();

        if let Err(error) = self.socket.send_to(&request, self.remote).await {
            return ProbeOutcome::Error(error.to_string());
        }

        let deadline = started + self.timeout;
        match self.recv_matching_reply(sequence, deadline).await {
            Ok((reply, peer)) => ProbeOutcome::Reply {
                rtt: started.elapsed(),
                peer: peer.to_string(),
                bytes: Some(reply.bytes),
                ttl: None,
                detail: vec![("gtp_seq".to_string(), reply.sequence.to_string())],
            },
            Err(outcome) => outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::net::UdpSocket;

    use super::*;

    #[tokio::test]
    async fn gtp_v1_probe_replies_to_local_echo_response() {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = server.local_addr().unwrap().port();
        let server_task = tokio::spawn(async move {
            let mut buf = [0_u8; 256];
            let (_len, peer) = server.recv_from(&mut buf).await.unwrap();
            let response = [0x32, 0x02, 0x00, 0x04, 0, 0, 0, 0, 0, 0, 0, 0];
            server.send_to(&response, peer).await.unwrap();
        });

        let mut prober = GtpProber::new(
            GtpVariant::V1u,
            "127.0.0.1".to_string(),
            Some(port),
            Duration::from_secs(1),
        )
        .await
        .unwrap();

        let outcome = prober.probe(0).await;
        assert!(matches!(outcome, ProbeOutcome::Reply { .. }));
        server_task.await.unwrap();
    }
}
