use std::{ffi::OsString, path::PathBuf, time::Duration};

use clap::{ArgAction, Parser, error::ErrorKind};

use crate::{cli::parse_seconds, protocol::icmp::external::ExternalPingConfig};

use super::native::NativeIcmpConfig;

#[derive(Debug)]
pub enum IcmpEngine {
    Native(NativeIcmpConfig),
    External(ExternalPingConfig),
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

    /// Destination hosts or IP addresses.
    #[arg(required = true, num_args = 1.., value_name = "DESTINATION")]
    destinations: Vec<String>,
}

impl From<NativeIcmpArgs> for NativeIcmpConfig {
    fn from(value: NativeIcmpArgs) -> Self {
        Self {
            destinations: value.destinations,
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
                assert_eq!(config.destinations, ["127.0.0.1"]);
            }
            IcmpEngine::External(_) => panic!("expected native engine"),
        }
    }

    #[test]
    fn parse_native_multiple_destinations() {
        let engine = parse_engine(vec![
            OsString::from("127.0.0.1"),
            OsString::from("127.0.0.2"),
        ])
        .unwrap();

        match engine {
            IcmpEngine::Native(config) => {
                assert_eq!(config.destinations, ["127.0.0.1", "127.0.0.2"]);
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
