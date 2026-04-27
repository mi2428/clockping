use std::{ffi::OsString, time::Duration};

use clap::{Args, Parser, Subcommand};

use crate::{timefmt::TimestampKind, version};

const ICMP_HELP: &str = "\
ICMP echo ping. Native by default; use --pinger to wrap system ping

Usage: clockping icmp [OPTIONS] <DESTINATION>...
       clockping icmp --pinger <PROGRAM> [PING_ARGS]...

Arguments:
  <DESTINATION>...  Destination host or IP address. Repeat for multiple targets
  [PING_ARGS]...    With --pinger, arguments passed unchanged to the external command

Options:
  -4                                      Use IPv4 only
  -6                                      Use IPv6 only
  -c, --count <COUNT>                     Stop after count probes. Default is to run until interrupted
  -i, --interval <SECONDS>                Seconds between probes. Fractions are accepted, e.g. 0.2 [default: 1]
  -W, --timeout <SECONDS>                 Per-probe timeout in seconds [default: 1]
  -w, --deadline <SECONDS>                Stop the command after this many seconds
  -s, --size <BYTES>                      Number of payload bytes [default: 56]
  -t, --ttl <TTL>                         IP TTL / hop limit
  -I, --interface-or-source <INTERFACE_OR_SOURCE>
                                          Interface name or source address
  -n, --numeric                           Numeric output only. Accepted for ping compatibility
  -q, --quiet                             Suppress per-probe output and only print the summary
  -D, --timestamp                         Accepted for ping compatibility. clockping timestamps every event by default
  -O, --report-outstanding                Report outstanding reply before sending next packet
      --pinger <PROGRAM>                  Run an external ping-compatible command instead of native ICMP
  -h, --help                              Print help
  -V, --version                           Print version

Output Options:
      --ts.preset <PRESET>              Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>              strftime-like timestamp format, similar to `date +\"...\"`
      --out.format <FORMAT>             Output format [default: text] [possible values: text, json]
      --out.colored                     Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>                    Push interval metrics to a Pushgateway URL
      --push.delete-on-exit               Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>          Aggregate interval samples before pushing window metrics
      --push.job <JOB>                    Pushgateway job name
      --push.label <KEY=VALUE>            Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>                  Retry failed Pushgateway requests N times
      --push.timeout <DURATION>           Pushgateway request timeout
      --push.user-agent <VALUE>           HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>               Write live interval metrics to a file
      --metrics.format <FORMAT>           Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>         Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>           Prometheus metric name prefix
";

#[derive(Debug, Parser)]
#[command(
    name = "clockping",
    version,
    long_version = version::LONG_VERSION,
    about = "A multi-protocol, multi-target pinger for watching hosts go dark",
    propagate_version = true
)]
pub struct Cli {
    /// Timestamp preset for human-readable output.
    #[arg(
        long = "ts.preset",
        value_enum,
        value_name = "PRESET",
        default_value_t = TimestampKind::Local,
        global = true,
        help_heading = "Output Options"
    )]
    pub timestamp: TimestampKind,

    /// strftime-like timestamp format, similar to `date +"..."`.
    #[arg(
        long = "ts.format",
        value_name = "FORMAT",
        global = true,
        help_heading = "Output Options"
    )]
    pub timestamp_format: Option<String>,

    /// Output format.
    #[arg(
        long = "out.format",
        value_enum,
        value_name = "FORMAT",
        default_value_t = OutputFormat::Text,
        global = true,
        help_heading = "Output Options"
    )]
    pub output_format: OutputFormat,

    /// Colorize human-readable output with ANSI escape sequences.
    #[arg(long = "out.colored", global = true, help_heading = "Output Options")]
    pub output_colored: bool,

    #[command(flatten)]
    pub metrics: MetricsCliOptions,

    #[command(subcommand)]
    pub command: Command,
}

// Metrics flags are stripped from raw argv before Clap parsing so environment
// defaults can be merged consistently. Keep this schema for help and completions.
#[allow(dead_code)]
#[derive(Debug, Args)]
#[command(next_help_heading = "Metrics Options")]
pub struct MetricsCliOptions {
    /// Push interval metrics to a Pushgateway URL.
    #[arg(long = "push.url", global = true, value_name = "URL")]
    pub push_url: Option<String>,

    /// Delete this Pushgateway grouping key after the run exits.
    #[arg(long = "push.delete-on-exit", global = true)]
    pub push_delete_on_exit: bool,

    /// Aggregate interval samples before pushing window metrics.
    #[arg(long = "push.interval", global = true, value_name = "DURATION")]
    pub push_interval: Option<String>,

    /// Pushgateway job name.
    #[arg(long = "push.job", global = true, value_name = "JOB")]
    pub push_job: Option<String>,

    /// Add a Pushgateway grouping label. Repeat for multiple labels.
    #[arg(long = "push.label", global = true, value_name = "KEY=VALUE")]
    pub push_labels: Vec<String>,

    /// Retry failed Pushgateway requests N times.
    #[arg(long = "push.retries", global = true, value_name = "N")]
    pub push_retries: Option<u32>,

    /// Pushgateway request timeout.
    #[arg(long = "push.timeout", global = true, value_name = "DURATION")]
    pub push_timeout: Option<String>,

    /// HTTP User-Agent for Pushgateway requests.
    #[arg(long = "push.user-agent", global = true, value_name = "VALUE")]
    pub push_user_agent: Option<String>,

    /// Write live interval metrics to a file.
    #[arg(long = "metrics.file", global = true, value_name = "PATH")]
    pub metrics_file: Option<String>,

    /// Metrics file format: jsonl or prometheus.
    #[arg(long = "metrics.format", global = true, value_name = "FORMAT")]
    pub metrics_format: Option<String>,

    /// Add a Prometheus file sample label. Repeat for multiple labels.
    #[arg(long = "metrics.label", global = true, value_name = "KEY=VALUE")]
    pub metrics_labels: Vec<String>,

    /// Prometheus metric name prefix.
    #[arg(long = "metrics.prefix", global = true, value_name = "PREFIX")]
    pub metrics_prefix: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl OutputFormat {
    pub fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// ICMP echo ping. Native by default; use --pinger to wrap system ping.
    Icmp(IcmpCommand),
    /// TCP connect ping.
    Tcp(TcpCommand),
    /// HTTP request ping. HEAD by default; use -X GET to send GET.
    Http(HttpCommand),
    /// GTP Echo ping.
    Gtp(GtpCommand),
    /// Generate a shell completion script.
    Completion(CompletionCommand),
}

#[derive(Debug, Args)]
pub struct CompletionCommand {
    /// Shell to generate completion script for.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

#[derive(Debug, Args)]
#[command(override_help = ICMP_HELP)]
pub struct IcmpCommand {
    /// ICMP native options, or system ping argv when --pinger is specified.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

#[derive(Debug, Args)]
pub struct TcpCommand {
    /// Use IPv4 only.
    #[arg(short = '4', conflicts_with = "ipv6")]
    pub ipv4: bool,

    /// Use IPv6 only.
    #[arg(short = '6')]
    pub ipv6: bool,

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

    /// Targets as host:port. Repeat for multiple targets.
    #[arg(required = true, num_args = 1.., value_name = "TARGET", value_parser = parse_tcp_target)]
    pub targets: Vec<String>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum HttpMethod {
    Head,
    Get,
}

#[derive(Clone, Debug)]
pub struct HeaderArg {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct StatusRanges {
    ranges: Vec<std::ops::RangeInclusive<u16>>,
}

impl StatusRanges {
    pub fn into_ranges(self) -> Vec<std::ops::RangeInclusive<u16>> {
        self.ranges
    }
}

#[derive(Debug, Args)]
pub struct HttpCommand {
    /// Use IPv4 only.
    #[arg(short = '4', conflicts_with = "ipv6")]
    pub ipv4: bool,

    /// Use IPv6 only.
    #[arg(short = '6')]
    pub ipv6: bool,

    /// Stop after count probes. Default is to run until interrupted.
    #[arg(short = 'c', long)]
    pub count: Option<u64>,

    /// Seconds between probes. Fractions are accepted, e.g. 0.2.
    #[arg(short = 'i', long, default_value = "1", value_parser = parse_seconds)]
    pub interval: Duration,

    /// Per-probe request timeout in seconds.
    #[arg(short = 'W', long, default_value = "1", value_parser = parse_seconds)]
    pub timeout: Duration,

    /// Stop the command after this many seconds.
    #[arg(short = 'w', long, value_parser = parse_seconds)]
    pub deadline: Option<Duration>,

    /// HTTP method to send.
    #[arg(short = 'X', long, value_enum, ignore_case = true, default_value_t = HttpMethod::Head)]
    pub method: HttpMethod,

    /// Treat these HTTP status codes as successful, e.g. 200,204,300-399.
    #[arg(long, default_value = "200-399", value_parser = parse_status_ranges)]
    pub ok_status: StatusRanges,

    /// Add a request header. Repeat for multiple headers.
    #[arg(short = 'H', long = "header", value_parser = parse_header)]
    pub headers: Vec<HeaderArg>,

    /// Follow HTTP redirects.
    #[arg(short = 'L', long = "location")]
    pub follow_redirects: bool,

    /// Skip TLS certificate verification.
    #[arg(short = 'k', long)]
    pub insecure: bool,

    /// Suppress per-probe output and only print the summary.
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// Target URLs. If no scheme is given, http:// is assumed.
    #[arg(required = true, num_args = 1.., value_name = "TARGET")]
    pub targets: Vec<String>,
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

    /// Target hosts or IP addresses.
    #[arg(required = true, num_args = 1.., value_name = "TARGET")]
    pub targets: Vec<String>,
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

fn parse_tcp_target(value: &str) -> Result<String, String> {
    crate::protocol::tcp::normalize_tcp_target(value).map_err(|error| error.to_string())
}

fn parse_header(value: &str) -> Result<HeaderArg, String> {
    let (name, header_value) = value
        .split_once(':')
        .ok_or_else(|| format!("expected header as 'Name: value', got {value:?}"))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("expected non-empty header name in {value:?}"));
    }
    Ok(HeaderArg {
        name: name.to_string(),
        value: header_value.trim_start().to_string(),
    })
}

fn parse_status_ranges(value: &str) -> Result<StatusRanges, String> {
    let mut ranges = Vec::new();
    for raw_part in value.split(',') {
        let part = raw_part.trim();
        if part.is_empty() {
            return Err(format!("empty status code in {value:?}"));
        }

        let (start, end) = match part.split_once('-') {
            Some((start, end)) => (parse_status_code(start)?, parse_status_code(end)?),
            None => {
                let code = parse_status_code(part)?;
                (code, code)
            }
        };
        if start > end {
            return Err(format!("status range start exceeds end in {part:?}"));
        }
        ranges.push(start..=end);
    }

    if ranges.is_empty() {
        return Err("expected at least one status code".to_string());
    }
    Ok(StatusRanges { ranges })
}

fn parse_status_code(value: &str) -> Result<u16, String> {
    let code = value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("expected HTTP status code, got {value:?}"))?;
    if !(100..=599).contains(&code) {
        return Err(format!(
            "expected HTTP status code between 100 and 599, got {code}"
        ));
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use clap_complete::Shell;

    use super::*;

    #[test]
    fn parse_fractional_seconds() {
        assert_eq!(parse_seconds("0.25").unwrap(), Duration::from_millis(250));
    }

    #[test]
    fn reject_negative_seconds() {
        assert!(parse_seconds("-1").is_err());
    }

    #[test]
    fn parse_header_splits_name_and_value() {
        let header = parse_header("User-Agent: clockping").unwrap();
        assert_eq!(header.name, "User-Agent");
        assert_eq!(header.value, "clockping");
    }

    #[test]
    fn parse_status_ranges_accepts_values_and_ranges() {
        let ranges = parse_status_ranges("200,204,300-399")
            .unwrap()
            .into_ranges();
        assert!(ranges.iter().any(|range| range.contains(&200)));
        assert!(ranges.iter().any(|range| range.contains(&204)));
        assert!(ranges.iter().any(|range| range.contains(&302)));
        assert!(!ranges.iter().any(|range| range.contains(&500)));
    }

    #[test]
    fn reject_invalid_status_ranges() {
        assert!(parse_status_ranges("99").is_err());
        assert!(parse_status_ranges("300-200").is_err());
        assert!(parse_status_ranges("").is_err());
    }

    #[test]
    fn parse_completion_shell() {
        let cli = Cli::parse_from(["clockping", "completion", "bash"]);

        assert!(matches!(
            cli.command,
            Command::Completion(CompletionCommand { shell: Shell::Bash })
        ));
    }

    #[test]
    fn tcp_accepts_multiple_targets() {
        let cli = Cli::parse_from(["clockping", "tcp", "one:443", "two:443"]);

        let Command::Tcp(command) = cli.command else {
            panic!("expected tcp command");
        };
        assert_eq!(command.targets, ["one:443", "two:443"]);
    }

    #[test]
    fn tcp_accepts_ip_version_flags() {
        let cli = Cli::parse_from(["clockping", "tcp", "-4", "one:443"]);

        let Command::Tcp(command) = cli.command else {
            panic!("expected tcp command");
        };
        assert!(command.ipv4);
        assert!(!command.ipv6);

        let cli = Cli::parse_from(["clockping", "tcp", "-6", "one:443"]);

        let Command::Tcp(command) = cli.command else {
            panic!("expected tcp command");
        };
        assert!(!command.ipv4);
        assert!(command.ipv6);
    }

    #[test]
    fn tcp_rejects_conflicting_ip_version_flags() {
        let error = Cli::try_parse_from(["clockping", "tcp", "-4", "-6", "one:443"]).unwrap_err();

        assert!(error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn http_accepts_ip_version_flags() {
        let cli = Cli::parse_from(["clockping", "http", "-4", "example.com"]);

        let Command::Http(command) = cli.command else {
            panic!("expected http command");
        };
        assert!(command.ipv4);
        assert!(!command.ipv6);

        let cli = Cli::parse_from(["clockping", "http", "-6", "example.com"]);

        let Command::Http(command) = cli.command else {
            panic!("expected http command");
        };
        assert!(!command.ipv4);
        assert!(command.ipv6);
    }

    #[test]
    fn http_rejects_conflicting_ip_version_flags() {
        let error =
            Cli::try_parse_from(["clockping", "http", "-4", "-6", "example.com"]).unwrap_err();

        assert!(error.to_string().contains("cannot be used with"));
    }

    #[test]
    fn tcp_rejects_target_without_port() {
        let error = Cli::try_parse_from(["clockping", "tcp", "example.com"]).unwrap_err();

        assert!(error.to_string().contains("TCP target must include a port"));
    }

    #[test]
    fn output_colored_is_global() {
        let cli = Cli::parse_from(["clockping", "tcp", "--out.colored", "one:443"]);

        assert!(cli.output_colored);
    }

    #[test]
    fn output_colored_is_global_for_icmp_raw_args() {
        let cli = Cli::parse_from(["clockping", "icmp", "--out.colored", "127.0.0.1"]);

        assert!(cli.output_colored);
        let Command::Icmp(command) = cli.command else {
            panic!("expected icmp command");
        };
        assert_eq!(command.args, [OsString::from("127.0.0.1")]);
    }

    #[test]
    fn icmp_timestamp_long_option_is_native_arg() {
        let cli = Cli::parse_from(["clockping", "icmp", "--timestamp", "127.0.0.1"]);

        let Command::Icmp(command) = cli.command else {
            panic!("expected icmp command");
        };
        assert_eq!(
            command.args,
            [OsString::from("--timestamp"), OsString::from("127.0.0.1")]
        );
    }

    #[test]
    fn output_options_are_global_for_modes() {
        let cli = Cli::parse_from([
            "clockping",
            "icmp",
            "--ts.preset",
            "none",
            "--out.format",
            "json",
            "127.0.0.1",
        ]);

        assert_eq!(cli.timestamp, TimestampKind::None);
        assert_eq!(cli.output_format, OutputFormat::Json);
        let Command::Icmp(command) = cli.command else {
            panic!("expected icmp command");
        };
        assert_eq!(command.args, [OsString::from("127.0.0.1")]);

        let cli = Cli::parse_from(["clockping", "tcp", "--ts.preset", "unix-ms", "one:443"]);

        assert_eq!(cli.timestamp, TimestampKind::UnixMs);
    }

    #[test]
    fn removed_output_option_aliases_are_rejected() {
        for argv in [
            &["clockping", "--timestamp", "none", "tcp", "one:443"][..],
            &["clockping", "--timestamp-format", "STAMP", "tcp", "one:443"][..],
            &["clockping", "--timestamp.preset", "none", "tcp", "one:443"][..],
            &["clockping", "--timestamp.format", "STAMP", "tcp", "one:443"][..],
            &["clockping", "--json", "tcp", "one:443"][..],
            &["clockping", "--colored", "tcp", "one:443"][..],
            &["clockping", "--output.format", "json", "tcp", "one:443"][..],
            &["clockping", "--output.color", "always", "tcp", "one:443"][..],
            &["clockping", "--out.color", "always", "tcp", "one:443"][..],
            &["clockping", "--out.colored", "always", "tcp", "one:443"][..],
        ] {
            assert!(Cli::try_parse_from(argv).is_err());
        }
    }
}
