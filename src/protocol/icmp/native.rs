use std::{
    net::{IpAddr, SocketAddr},
    num::NonZeroU32,
    sync::atomic::{AtomicU16, Ordering},
    time::Duration,
};

#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "ios",
    target_os = "visionos",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "illumos",
    target_os = "solaris",
))]
use std::ffi::CString;

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
        let mut interface_index = None;
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
                let index = interface_index_from_name(interface_or_source)?;
                builder = bind_interface(builder, interface_or_source, index)?;
                interface_index = Some(index);
            }
        }

        let client = Client::new(&builder.build())?;
        let ident = next_ping_identifier();
        let mut pinger = client.pinger(host, ident).await;
        if host.is_ipv6()
            && let Some(interface_index) = interface_index
        {
            pinger.scope_id(interface_index.get());
        }
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

fn bind_interface(
    builder: surge_ping::ConfigBuilder,
    interface: &str,
    interface_index: NonZeroU32,
) -> anyhow::Result<surge_ping::ConfigBuilder> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        let _ = interface_index;
        Ok(builder.interface(interface))
    }

    #[cfg(any(
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "illumos",
        target_os = "solaris",
    ))]
    {
        let _ = interface;
        Ok(builder.interface_index(interface_index))
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "linux",
        target_os = "ios",
        target_os = "visionos",
        target_os = "macos",
        target_os = "tvos",
        target_os = "watchos",
        target_os = "illumos",
        target_os = "solaris",
    )))]
    {
        let _ = builder;
        let _ = interface;
        let _ = interface_index;
        anyhow::bail!(
            "interface selection by name is not supported on this platform; use a source address with -I"
        );
    }
}

#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "ios",
    target_os = "visionos",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "illumos",
    target_os = "solaris",
))]
fn interface_index_from_name(interface: &str) -> anyhow::Result<NonZeroU32> {
    let interface = CString::new(interface)?;
    let index = unsafe { libc::if_nametoindex(interface.as_ptr()) };
    NonZeroU32::new(index).ok_or_else(|| {
        anyhow::anyhow!("unknown network interface: {}", interface.to_string_lossy())
    })
}

#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "ios",
    target_os = "visionos",
    target_os = "macos",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "illumos",
    target_os = "solaris",
)))]
fn interface_index_from_name(_interface: &str) -> anyhow::Result<NonZeroU32> {
    anyhow::bail!(
        "interface selection by name is not supported on this platform; use a source address with -I"
    )
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
        ping_outcome(
            self.pinger.ping(ping_seq, &self.payload).await,
            ping_seq,
            self.report_outstanding,
        )
    }
}

fn ping_outcome(
    result: Result<(IcmpPacket, Duration), surge_ping::SurgeError>,
    ping_seq: PingSequence,
    report_outstanding: bool,
) -> ProbeOutcome {
    // surge-ping also delivers router errors as Ok. Only Echo Reply with code 0 is up.
    match result {
        Ok((IcmpPacket::V4(packet), rtt))
            if packet.get_icmp_type().0 == 0 && packet.get_icmp_code().0 == 0 =>
        {
            ProbeOutcome::Reply {
                rtt,
                peer: packet.get_source().to_string(),
                bytes: Some(packet.get_size()),
                ttl: packet.get_ttl(),
                detail: vec![("icmp_seq".to_string(), packet.get_sequence().0.to_string())],
            }
        }
        Ok((IcmpPacket::V6(packet), rtt))
            if packet.get_icmpv6_type().0 == 129 && packet.get_icmpv6_code().0 == 0 =>
        {
            ProbeOutcome::Reply {
                rtt,
                peer: packet.get_source().to_string(),
                bytes: Some(packet.get_size()),
                ttl: Some(packet.get_max_hop_limit()),
                detail: vec![("icmp_seq".to_string(), packet.get_sequence().0.to_string())],
            }
        }
        Ok((packet, _)) => {
            let (family, kind, code, source) = match &packet {
                IcmpPacket::V4(packet) => (
                    "ICMPv4",
                    packet.get_icmp_type().0,
                    packet.get_icmp_code().0,
                    packet.get_source().to_string(),
                ),
                IcmpPacket::V6(packet) => (
                    "ICMPv6",
                    packet.get_icmpv6_type().0,
                    packet.get_icmpv6_code().0,
                    packet.get_source().to_string(),
                ),
            };
            let description = match (family, kind) {
                ("ICMPv4", 3) | ("ICMPv6", 1) => "destination unreachable",
                ("ICMPv4", 11) | ("ICMPv6", 3) => "time exceeded",
                ("ICMPv4", 12) | ("ICMPv6", 4) => "parameter problem",
                ("ICMPv4", 5) => "redirect",
                ("ICMPv6", 2) => "packet too big",
                _ => "unexpected packet",
            };
            ProbeOutcome::Error(format!(
                "{family} {description} from {source} (type={kind} code={code} icmp_seq={})",
                packet.get_sequence().0,
            ))
        }
        Err(surge_ping::SurgeError::Timeout { .. }) => {
            let detail = if report_outstanding {
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

#[cfg(test)]
mod tests {
    use std::{future::Future, task::Poll};

    use chrono::Local;
    use socket2::Type;
    use surge_ping::{Icmpv4Packet, Icmpv6Packet, SurgeError};

    use crate::{event::ProbeEvent, metrics::ProbeMetrics, runner::Summary};

    use super::*;

    const RTT: Duration = Duration::from_millis(12);
    const SEQ: PingSequence = PingSequence(300);

    fn echo(kind: u8, code: u8) -> Vec<u8> {
        let mut bytes = vec![kind, code, 0, 0, 0xbe, 0xef];
        bytes.extend_from_slice(&SEQ.0.to_be_bytes());
        bytes.extend_from_slice(&[1, 2, 3, 4]);
        bytes
    }

    fn ipv4_header(payload: &[u8], options: bool) -> Vec<u8> {
        let mut bytes = vec![if options { 0x47 } else { 0x45 }, 0];
        bytes.extend_from_slice(
            &((20 + if options { 8 } else { 0 } + payload.len()) as u16).to_be_bytes(),
        );
        bytes.extend_from_slice(&[0, 0, 0, 0, 73, 1, 0, 0, 192, 0, 2, 1, 198, 51, 100, 7]);
        if options {
            bytes.extend_from_slice(&[7, 8, 4, 0, 0, 0, 0, 0]);
        }
        bytes.extend_from_slice(payload);
        bytes
    }

    fn decode_v4(body: &[u8], socket_type: Type) -> Result<IcmpPacket, SurgeError> {
        let bytes = if surge_ping::is_linux_icmp_socket!(socket_type) {
            body.to_vec()
        } else {
            ipv4_header(body, false)
        };
        Icmpv4Packet::decode(
            &bytes,
            socket_type,
            "192.0.2.1".parse().unwrap(),
            "198.51.100.7".parse().unwrap(),
        )
        .map(IcmpPacket::V4)
    }

    fn decode_v6(body: &[u8]) -> Result<IcmpPacket, SurgeError> {
        Icmpv6Packet::decode(body, "2001:db8::1".parse().unwrap()).map(IcmpPacket::V6)
    }

    fn quoted_error(v6: bool, kind: u8, code: u8, options: bool) -> Vec<u8> {
        let mut bytes = vec![kind, code, 0, 0, 0, 0, 0, 0];
        let request = echo(if v6 { 128 } else { 8 }, 0);
        if v6 {
            bytes.extend_from_slice(&[
                0x60,
                0,
                0,
                0,
                0,
                if options { 20 } else { 12 },
                if options { 60 } else { 58 },
                64,
            ]);
            bytes.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
            bytes.extend_from_slice(
                &"2001:db8::7"
                    .parse::<std::net::Ipv6Addr>()
                    .unwrap()
                    .octets(),
            );
            if options {
                bytes.extend_from_slice(&[58, 0, 1, 4, 0, 0, 0, 0]);
            }
            bytes.extend_from_slice(&request);
        } else {
            bytes.extend_from_slice(&ipv4_header(&request, options));
        }
        bytes
    }

    fn assert_lost(outcome: ProbeOutcome, status: &str) {
        let ts = Local::now();
        let mut summary = Summary::new("target".to_string());
        assert!(summary.record(ts, &outcome).is_none());
        let event = ProbeEvent {
            ts,
            protocol: "icmp",
            target: summary.target.clone(),
            seq: SEQ.0 as u64,
            outcome,
            recovery: None,
        };
        let metrics = ProbeMetrics::from_event(&event, &summary);
        assert_eq!(metrics.status, status);
        assert_eq!((metrics.sent, metrics.received, metrics.lost), (1, 0, 1));
        assert_eq!((metrics.up, metrics.loss_pct), (0.0, 100.0));
        assert_eq!(
            (metrics.rtt_seconds, metrics.bytes, metrics.ttl),
            (None, None, None)
        );
        let json = serde_json::to_value(event.as_json(String::new())).unwrap();
        assert_eq!(json["status"], status);
        for field in ["rtt_ms", "peer", "bytes", "ttl"] {
            assert!(json.get(field).is_none(), "{json}");
        }
        assert_eq!(summary.rtt_min_avg_max(), None);
        assert_eq!(
            crate::exit_code_for_summary(&summary),
            std::process::ExitCode::FAILURE
        );

        let reply = ping_outcome(
            decode_v6(&echo(129, 0)).map(|packet| (packet, RTT)),
            SEQ,
            false,
        );
        let recovery = summary.record(ts, &reply).unwrap();
        assert_eq!(recovery.lost, 1);
        summary.finalize();
        assert_eq!(summary.loss_periods[0].lost, 1);
        assert_eq!(summary.loss_periods[0].end, Some(ts));
        assert_eq!((summary.sent, summary.received), (2, 1));
        assert_eq!(summary.rtt_min_avg_max(), Some((RTT, RTT, RTT)));
        assert_eq!(
            crate::exit_code_for_summary(&summary),
            std::process::ExitCode::SUCCESS
        );
    }

    #[test]
    fn echo_replies_preserve_fields_and_success_boundaries() {
        for (packet, peer, ttl) in [
            (
                decode_v4(&echo(0, 0), Type::RAW).unwrap(),
                "192.0.2.1",
                Some(73),
            ),
            (
                decode_v4(&echo(0, 0), Type::DGRAM).unwrap(),
                "192.0.2.1",
                if cfg!(any(target_os = "linux", target_os = "android")) {
                    None
                } else {
                    Some(73)
                },
            ),
            (decode_v6(&echo(129, 0)).unwrap(), "2001:db8::1", Some(0)),
        ] {
            assert_eq!(packet.get_identifier(), PingIdentifier(0xbeef));
            assert_eq!(packet.real_destination().to_string(), peer);
            let outcome = ping_outcome(Ok((packet, RTT)), SEQ, true);
            let ProbeOutcome::Reply {
                rtt,
                peer: actual_peer,
                bytes,
                ttl: actual_ttl,
                detail,
            } = &outcome
            else {
                panic!("expected EchoReply, got {outcome:?}");
            };
            assert_eq!(
                (*rtt, actual_peer.as_str(), *bytes, *actual_ttl),
                (RTT, peer, Some(12), ttl)
            );
            assert_eq!(detail, &[("icmp_seq".to_string(), "300".to_string())]);
            let ts = Local::now();
            let mut summary = Summary::new("target".to_string());
            summary.record(ts, &outcome);
            let event = ProbeEvent {
                ts,
                protocol: "icmp",
                target: summary.target.clone(),
                seq: 300,
                outcome,
                recovery: None,
            };
            let metrics = ProbeMetrics::from_event(&event, &summary);
            assert_eq!((metrics.sent, metrics.received, metrics.lost), (1, 1, 0));
            assert_eq!(
                (metrics.status, metrics.up, metrics.loss_pct),
                ("reply", 1.0, 0.0)
            );
            assert_eq!(
                (metrics.rtt_seconds, metrics.bytes, metrics.ttl),
                (Some(0.012), Some(12), ttl)
            );
            let json = serde_json::to_value(event.as_json(String::new())).unwrap();
            assert_eq!(json["status"], "reply");
            assert_eq!(json["peer"], peer);
            assert_eq!(json["bytes"], 12);
            assert_eq!(json["rtt_ms"], 12.0);
            assert_eq!(
                crate::exit_code_for_summary(&summary),
                std::process::ExitCode::SUCCESS
            );
        }
    }

    #[test]
    fn router_errors_and_invalid_echo_codes_are_never_success() {
        for (v6, kind, code, description) in [
            (false, 3, 1, "destination unreachable"),
            (false, 11, 0, "time exceeded"),
            (false, 11, 1, "time exceeded"),
            (false, 12, 0, "parameter problem"),
            (false, 5, 1, "redirect"),
            (true, 1, 3, "destination unreachable"),
            (true, 3, 0, "time exceeded"),
            (true, 3, 1, "time exceeded"),
            (true, 2, 0, "packet too big"),
            (true, 4, 0, "parameter problem"),
            (false, 42, 255, "unexpected packet"),
            (true, 42, 255, "unexpected packet"),
        ] {
            for options in [false, true] {
                for socket_type in [Type::RAW, Type::DGRAM] {
                    let body = quoted_error(v6, kind, code, options);
                    let packet = if v6 {
                        decode_v6(&body)
                    } else {
                        decode_v4(&body, socket_type)
                    }
                    .unwrap();
                    assert_eq!(
                        packet.real_destination().to_string(),
                        if v6 { "2001:db8::7" } else { "198.51.100.7" }
                    );
                    assert_eq!(packet.get_identifier(), PingIdentifier(0xbeef));
                    assert_eq!(packet.get_sequence(), SEQ);
                    let outcome = ping_outcome(Ok((packet, RTT)), SEQ, true);
                    let ProbeOutcome::Error(error) = &outcome else {
                        panic!("false successful router reply: {outcome:?}");
                    };
                    assert_eq!(
                        error,
                        &format!(
                            "{} {description} from {} (type={kind} code={code} icmp_seq=300)",
                            if v6 { "ICMPv6" } else { "ICMPv4" },
                            if v6 { "2001:db8::1" } else { "192.0.2.1" }
                        )
                    );
                    assert_lost(outcome, "error");
                }
            }
        }
        for packet in [decode_v4(&echo(0, 1), Type::RAW), decode_v6(&echo(129, 1))] {
            assert_lost(
                ping_outcome(packet.map(|packet| (packet, RTT)), SEQ, false),
                "error",
            );
        }
    }

    #[test]
    fn malformed_packets_and_timeout_preserve_loss_and_outstanding() {
        for v6 in [false, true] {
            for body in [
                echo(if v6 { 129 } else { 0 }, 0),
                quoted_error(v6, if v6 { 3 } else { 11 }, 0, true),
            ] {
                let header_end = body.len() - 4;
                for len in 0..header_end {
                    let result = if v6 {
                        decode_v6(&body[..len])
                    } else {
                        decode_v4(&body[..len], Type::RAW)
                    };
                    assert!(result.is_err(), "decoded truncated packet: {result:?}");
                    assert_lost(
                        ping_outcome(result.map(|packet| (packet, RTT)), SEQ, true),
                        "error",
                    );
                }
            }
        }
        let mut wrong_protocol = quoted_error(true, 3, 0, false);
        wrong_protocol[14] = 17;
        assert!(decode_v6(&wrong_protocol).is_err());
        let mut invalid_ihl = quoted_error(false, 11, 0, false);
        invalid_ihl[8] = 0x44;
        assert!(decode_v4(&invalid_ihl, Type::RAW).is_err());

        for outstanding in [false, true] {
            let outcome = ping_outcome(Err(SurgeError::Timeout { seq: SEQ }), SEQ, outstanding);
            let ProbeOutcome::Timeout { detail } = &outcome else {
                panic!("expected timeout");
            };
            assert_eq!(
                detail,
                &if outstanding {
                    vec![
                        ("icmp_seq".to_string(), "300".to_string()),
                        ("outstanding".to_string(), "true".to_string()),
                    ]
                } else {
                    Vec::new()
                }
            );
            assert_lost(outcome, "timeout");
        }
    }

    #[tokio::test]
    #[ignore = "requires native IPv4/IPv6 sockets; run explicitly alongside tests/icmp_smoke.py"]
    async fn native_loopback_timeout_cancellation_and_client_lifetime() {
        for host in ["127.0.0.1", "::1"] {
            let super::super::args::IcmpEngine::Native(config) = super::super::args::parse_engine(
                ["-n", "-W", "2", host]
                    .into_iter()
                    .map(std::ffi::OsString::from)
                    .collect(),
            )
            .unwrap() else {
                panic!("expected native config");
            };
            let mut prober = NativeIcmpProber::new(config).await.unwrap();
            eprintln!(
                "{host}: native socket {:?}",
                prober._client.get_socket().get_type()
            );
            drop(prober._client.clone());
            assert!(matches!(prober.probe(0).await, ProbeOutcome::Reply { .. }));
            // Invert the public identifier mode so loopback replies cannot match the waiter.
            // This forces real timeouts without an external blackhole or network reconfiguration.
            let identifier = prober.pinger.ident;
            assert_eq!(
                identifier.is_none(),
                surge_ping::is_linux_icmp_socket!(prober._client.get_socket().get_type())
            );
            prober.pinger.ident = if identifier.is_some() {
                None
            } else {
                Some(next_ping_identifier())
            };
            prober.pinger.timeout(Duration::from_millis(30));
            prober.report_outstanding = true;
            // Poll once on this single-thread runtime, then cancel before recv_task can deliver.
            let mut pending = Box::pin(prober.probe(300));
            std::future::poll_fn(|cx| {
                assert!(pending.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            drop(pending);
            for _ in 0..2 {
                let outcome = prober.probe(300).await;
                let ProbeOutcome::Timeout { detail } = &outcome else {
                    panic!("cancelled/timed-out sequence was not reusable for {host}: {outcome:?}");
                };
                assert_eq!(
                    detail,
                    &[
                        ("icmp_seq".to_string(), "300".to_string()),
                        ("outstanding".to_string(), "true".to_string())
                    ]
                );
                assert_lost(outcome, "timeout");
            }
            prober.pinger.ident = identifier;
            prober.pinger.timeout(Duration::from_secs(2));
            assert!(matches!(
                prober.probe(301).await,
                ProbeOutcome::Reply { .. }
            ));
            let ProbeOutcome::Reply { detail, .. } = prober.probe(65536).await else {
                panic!("wrapped sequence did not reply for {host}");
            };
            assert_eq!(detail, [("icmp_seq".to_string(), "0".to_string())]);

            let mut orphan = prober
                ._client
                .pinger(host.parse().unwrap(), next_ping_identifier())
                .await;
            let mut pending = Box::pin(orphan.ping(SEQ, &[0; 8]));
            std::future::poll_fn(|cx| {
                assert!(pending.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            drop(prober);
            assert!(matches!(pending.await, Err(SurgeError::ClientDestroyed)));
        }
    }

    #[test]
    fn ping_identifiers_are_unique_per_native_prober() {
        assert_ne!(next_ping_identifier().0, next_ping_identifier().0);
    }

    #[test]
    fn invalid_interface_name_is_rejected() {
        let error = interface_index_from_name("clockping-invalid-interface").unwrap_err();
        assert_eq!(
            error.to_string(),
            "unknown network interface: clockping-invalid-interface"
        );
    }
}
