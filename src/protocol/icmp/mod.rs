use std::{
    ffi::OsString,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use anyhow::Context;
use async_trait::async_trait;
use clap::{ArgAction, Parser, error::ErrorKind};
use surge_ping::{Client, Config, ICMP, IcmpPacket, PingIdentifier, PingSequence};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::lookup_host,
    process::Command,
};

use crate::{cli::parse_seconds, event::ProbeOutcome, output::Output, runner::Prober};

#[derive(Debug)]
pub enum IcmpEngine {
    Native(NativeIcmpConfig),
    External(ExternalPingConfig),
}

#[derive(Debug, Clone)]
pub struct ExternalPingConfig {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone)]
pub struct NativeIcmpConfig {
    pub destination: String,
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

#[derive(Debug, Parser)]
#[command(name = "clockping icmp", disable_help_subcommand = true)]
struct NativeIcmpArgs {
    /// Use IPv4 only.
    #[arg(short = '4', action = ArgAction::SetTrue)]
    ipv4: bool,

    /// Use IPv6 only.
    #[arg(short = '6', action = ArgAction::SetTrue)]
    ipv6: bool,

    /// Stop after count probes. Default is to run until interrupted.
    #[arg(short = 'c', long)]
    count: Option<u64>,

    /// Seconds between probes. Fractions are accepted, e.g. 0.2.
    #[arg(short = 'i', long, default_value = "1", value_parser = parse_seconds)]
    interval: Duration,

    /// Per-probe timeout in seconds.
    #[arg(short = 'W', long, default_value = "1", value_parser = parse_seconds)]
    timeout: Duration,

    /// Stop the command after this many seconds.
    #[arg(short = 'w', long, value_parser = parse_seconds)]
    deadline: Option<Duration>,

    /// Number of payload bytes.
    #[arg(short = 's', long, default_value_t = 56)]
    size: usize,

    /// IP TTL / hop limit.
    #[arg(short = 't', long)]
    ttl: Option<u32>,

    /// Interface name or source address.
    #[arg(short = 'I', long)]
    interface_or_source: Option<String>,

    /// Numeric output only. Accepted for ping compatibility.
    #[arg(short = 'n', long)]
    numeric: bool,

    /// Quiet output.
    #[arg(short = 'q', long)]
    quiet: bool,

    /// Accepted for ping compatibility. clockping timestamps every event by default.
    #[arg(short = 'D', long)]
    timestamp: bool,

    /// Report outstanding reply before sending next packet.
    #[arg(short = 'O', long)]
    report_outstanding: bool,

    /// Destination host or IP address.
    destination: String,
}

impl From<NativeIcmpArgs> for NativeIcmpConfig {
    fn from(value: NativeIcmpArgs) -> Self {
        Self {
            destination: value.destination,
            ipv4: value.ipv4,
            ipv6: value.ipv6,
            count: value.count,
            interval: value.interval,
            timeout: value.timeout,
            deadline: value.deadline,
            size: value.size,
            ttl: value.ttl,
            interface_or_source: value.interface_or_source,
            numeric: value.numeric,
            quiet: value.quiet,
            timestamp: value.timestamp,
            report_outstanding: value.report_outstanding,
        }
    }
}

pub fn parse_engine(args: Vec<OsString>) -> anyhow::Result<IcmpEngine> {
    let (pinger, remaining) = extract_pinger(args)?;
    if let Some(program) = pinger {
        return Ok(IcmpEngine::External(ExternalPingConfig {
            program,
            args: remaining,
        }));
    }

    let mut argv = Vec::with_capacity(remaining.len() + 1);
    argv.push(OsString::from("clockping icmp"));
    argv.extend(remaining);
    match NativeIcmpArgs::try_parse_from(argv) {
        Ok(args) => Ok(IcmpEngine::Native(args.into())),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.exit()
        }
        Err(error) => Err(anyhow::anyhow!(
            "{error}\nhint: use `clockping icmp --pinger=/usr/bin/ping ...` for ping options that native mode does not support yet"
        )),
    }
}

fn extract_pinger(args: Vec<OsString>) -> anyhow::Result<(Option<PathBuf>, Vec<OsString>)> {
    let mut pinger = None;
    let mut remaining = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        let Some(text) = arg.to_str() else {
            remaining.push(arg);
            continue;
        };

        if text == "--pinger" {
            let value = iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("--pinger requires a program path"))?;
            pinger = Some(PathBuf::from(value));
            continue;
        }

        if let Some(value) = text.strip_prefix("--pinger=") {
            pinger = Some(PathBuf::from(value));
            continue;
        }

        remaining.push(arg);
    }

    Ok((pinger, remaining))
}

pub async fn run_external(config: ExternalPingConfig, output: Output) -> anyhow::Result<()> {
    let mut child = Command::new(&config.program)
        .args(&config.args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", config.program.display()))?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let stderr = child.stderr.take().context("failed to capture stderr")?;
    let stdout_output = output.clone();
    let stderr_output = output.clone();

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            stdout_output.print_external_line("stdout", &line)?;
        }
        anyhow::Ok(())
    });

    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Some(line) = lines.next_line().await? {
            stderr_output.print_external_line("stderr", &line)?;
        }
        anyhow::Ok(())
    });

    let status = child.wait().await?;
    stdout_task.await??;
    stderr_task.await??;
    if !status.success() {
        anyhow::bail!("{} exited with {status}", config.program.display());
    }
    Ok(())
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
        let host = resolve_icmp_host(&config.destination, config.ipv4, config.ipv6).await?;
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
        let ident = PingIdentifier((std::process::id() & 0xffff) as u16);
        let mut pinger = client.pinger(host, ident).await;
        pinger.timeout(config.timeout);

        Ok(Self {
            target: if config.numeric {
                host.to_string()
            } else {
                format!("{} ({host})", config.destination)
            },
            _client: client,
            pinger,
            payload: vec![0; config.size],
            report_outstanding: config.report_outstanding,
        })
    }
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
    fn parse_pinger_equals_mode() {
        let engine = parse_engine(vec![
            OsString::from("--pinger=/usr/bin/ping"),
            OsString::from("-w"),
            OsString::from("1"),
            OsString::from("127.0.0.1"),
        ])
        .unwrap();

        match engine {
            IcmpEngine::External(config) => {
                assert_eq!(config.program, PathBuf::from("/usr/bin/ping"));
                assert_eq!(config.args, vec!["-w", "1", "127.0.0.1"]);
            }
            IcmpEngine::Native(_) => panic!("expected external engine"),
        }
    }

    #[test]
    fn parse_native_count_and_destination() {
        let engine = parse_engine(vec![
            OsString::from("-c"),
            OsString::from("3"),
            OsString::from("-i"),
            OsString::from("0.2"),
            OsString::from("127.0.0.1"),
        ])
        .unwrap();

        match engine {
            IcmpEngine::Native(config) => {
                assert_eq!(config.count, Some(3));
                assert_eq!(config.interval, Duration::from_millis(200));
                assert_eq!(config.destination, "127.0.0.1");
            }
            IcmpEngine::External(_) => panic!("expected native engine"),
        }
    }

    #[test]
    fn parse_native_compatibility_flags() {
        let engine = parse_engine(vec![
            OsString::from("-n"),
            OsString::from("-D"),
            OsString::from("-O"),
            OsString::from("127.0.0.1"),
        ])
        .unwrap();

        match engine {
            IcmpEngine::Native(config) => {
                assert!(config.numeric);
                assert!(config.timestamp);
                assert!(config.report_outstanding);
            }
            IcmpEngine::External(_) => panic!("expected native engine"),
        }
    }

    #[test]
    fn parse_native_rejects_unknown_option_with_hint() {
        let error = parse_engine(vec![OsString::from("-M"), OsString::from("do")])
            .unwrap_err()
            .to_string();
        assert!(error.contains("--pinger=/usr/bin/ping"));
    }
}
