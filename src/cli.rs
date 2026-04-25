use std::{ffi::OsString, time::Duration};

use clap::{Args, Parser, Subcommand};

use crate::timefmt::TimestampKind;

#[derive(Debug, Parser)]
#[command(name = "clockping", version, about = "Timestamped generic pinger")]
pub struct Cli {
    /// Timestamp preset for human-readable output.
    #[arg(long, value_enum, default_value_t = TimestampKind::Local)]
    pub timestamp: TimestampKind,

    /// strftime-like timestamp format, similar to `date +"..."`.
    #[arg(long)]
    pub timestamp_format: Option<String>,

    /// Emit JSON Lines instead of text.
    #[arg(long)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// ICMP echo ping. Native by default; use --pinger to wrap system ping.
    Icmp(IcmpCommand),
    /// TCP connect ping.
    Tcp(TcpCommand),
    /// GTP Echo ping.
    Gtp(GtpCommand),
}

#[derive(Debug, Args)]
#[command(
    after_help = "Native ICMP options are parsed after this raw argv layer:
  -4 -6 -c <count> -i <seconds> -W <seconds> -w <seconds>
  -s <bytes> -t <ttl> -I <interface-or-source> -n -q -D -O <destination>

External wrapper mode:
  clockping icmp --pinger=/usr/bin/ping [PING_ARGS...]"
)]
pub struct IcmpCommand {
    /// ICMP native options, or system ping argv when --pinger is specified.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

#[derive(Debug, Args)]
pub struct TcpCommand {
    /// Stop after count probes. Default is to run until interrupted.
    #[arg(short = 'c', long)]
    pub count: Option<u64>,

    /// Seconds between probes. Fractions are accepted, e.g. 0.2.
    #[arg(short = 'i', long, default_value = "1", value_parser = parse_seconds)]
    pub interval: Duration,

    /// Per-probe connect timeout in seconds.
    #[arg(short = 'W', long, default_value = "1", value_parser = parse_seconds)]
    pub timeout: Duration,

    /// Stop the command after this many seconds.
    #[arg(short = 'w', long, value_parser = parse_seconds)]
    pub deadline: Option<Duration>,

    /// Suppress per-probe output and only print the summary.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Target as host:port.
    pub target: String,
}

#[derive(Debug, Args)]
pub struct GtpCommand {
    #[command(subcommand)]
    pub command: GtpSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum GtpSubcommand {
    /// GTPv1-U Echo Request, default UDP/2152.
    V1u(GtpProbeArgs),
    /// GTPv1-C Echo Request, default UDP/2123.
    V1c(GtpProbeArgs),
    /// GTPv2-C Echo Request, default UDP/2123.
    V2c(GtpProbeArgs),
}

#[derive(Debug, Args)]
pub struct GtpProbeArgs {
    /// Stop after count probes. Default is to run until interrupted.
    #[arg(short = 'c', long)]
    pub count: Option<u64>,

    /// Seconds between probes. Fractions are accepted, e.g. 0.2.
    #[arg(short = 'i', long, default_value = "1", value_parser = parse_seconds)]
    pub interval: Duration,

    /// Per-probe response timeout in seconds.
    #[arg(short = 'W', long, default_value = "1", value_parser = parse_seconds)]
    pub timeout: Duration,

    /// Stop the command after this many seconds.
    #[arg(short = 'w', long, value_parser = parse_seconds)]
    pub deadline: Option<Duration>,

    /// Override the protocol default UDP port.
    #[arg(long)]
    pub port: Option<u16>,

    /// Suppress per-probe output and only print the summary.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Target host or IP address.
    pub target: String,
}

pub fn parse_seconds(value: &str) -> Result<Duration, String> {
    let seconds = value
        .parse::<f64>()
        .map_err(|_| format!("expected seconds as a number, got {value:?}"))?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(format!(
            "expected a non-negative finite duration, got {value:?}"
        ));
    }
    Ok(Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fractional_seconds() {
        assert_eq!(parse_seconds("0.25").unwrap(), Duration::from_millis(250));
    }

    #[test]
    fn reject_negative_seconds() {
        assert!(parse_seconds("-1").is_err());
    }
}
