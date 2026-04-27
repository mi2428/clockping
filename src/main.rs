mod cli;
mod event;
mod metrics;
mod metrics_file;
mod metrics_options;
mod output;
mod prometheus;
mod protocol;
mod pushgateway;
mod runner;
mod timefmt;
mod version;

use std::{
    io::{self, Write},
    process::ExitCode,
};

use anyhow::Context;
use clap::{CommandFactory, Parser};
use clap_complete::generate;

use crate::{
    cli::{Cli, Command},
    metrics_options::extract_metrics_options,
    output::Output,
    protocol::{
        gtp::{GtpProber, GtpVariant},
        http::{HttpProber, HttpProberConfig},
        icmp::{self, IcmpEngine},
        tcp::TcpProber,
    },
    runner::{RunnerConfig, Summary, run_probe_loop},
    timefmt::{TimestampFormatter, TimestampKind},
};

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) if output::is_broken_pipe(&error) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error:?}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<ExitCode> {
    let (metrics_options, cli_args) = extract_metrics_options(std::env::args_os().collect())?;
    let metrics_enabled = metrics_options.is_enabled();
    let mut metrics_options = Some(metrics_options);
    let cli = Cli::parse_from(cli_args);
    let timestamp = cli.timestamp;
    let timestamp_format = cli.timestamp_format;
    let json = cli.json;
    let mut exit_code = ExitCode::SUCCESS;

    match cli.command {
        Command::Completion(command) => {
            let mut cli_command = Cli::command();
            let mut script = Vec::new();
            generate(command.shell, &mut cli_command, "clockping", &mut script);
            io::stdout().write_all(&script)?;
        }
        Command::Icmp(command) => match icmp::parse_engine(command.args)? {
            IcmpEngine::External(external) => {
                if metrics_enabled {
                    anyhow::bail!(
                        "--push and --metrics are not supported with external --pinger mode"
                    );
                }
                let output = make_output(timestamp, timestamp_format.clone(), json, false);
                icmp::run_external(external, output).await?;
            }
            IcmpEngine::Native(config) => {
                let mut metrics = metrics_options
                    .take()
                    .expect("metrics options should be consumed once")
                    .into_reporter()?;
                let quiet = config.quiet;
                let output =
                    make_output(timestamp, timestamp_format.clone(), json, config.timestamp);
                let runner_config = RunnerConfig {
                    interval: config.interval,
                    count: config.count,
                    deadline: config.deadline,
                };
                let prober = icmp::NativeIcmpProber::new(config)
                    .await
                    .context("failed to initialize native ICMP prober")?;
                let summary =
                    run_probe_loop(prober, runner_config, output, quiet, metrics.as_mut()).await?;
                exit_code = exit_code_for_summary(&summary);
            }
        },
        Command::Tcp(command) => {
            let mut metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?;
            let quiet = command.quiet;
            let output = make_output(timestamp, timestamp_format.clone(), json, false);
            let runner_config = RunnerConfig {
                interval: command.interval,
                count: command.count,
                deadline: command.deadline,
            };
            let prober = TcpProber::new(command.target, command.timeout)
                .await
                .context("failed to initialize TCP prober")?;
            let summary =
                run_probe_loop(prober, runner_config, output, quiet, metrics.as_mut()).await?;
            exit_code = exit_code_for_summary(&summary);
        }
        Command::Http(command) => {
            let mut metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?;
            let quiet = command.quiet;
            let output = make_output(timestamp, timestamp_format.clone(), json, false);
            let runner_config = RunnerConfig {
                interval: command.interval,
                count: command.count,
                deadline: command.deadline,
            };
            let prober = HttpProber::new(HttpProberConfig {
                target: command.target,
                method: match command.method {
                    cli::HttpMethod::Head => reqwest::Method::HEAD,
                    cli::HttpMethod::Get => reqwest::Method::GET,
                },
                timeout: command.timeout,
                headers: command
                    .headers
                    .into_iter()
                    .map(|header| (header.name, header.value))
                    .collect(),
                follow_redirects: command.follow_redirects,
                insecure: command.insecure,
                ok_statuses: command.ok_status.into_ranges(),
            })
            .context("failed to initialize HTTP prober")?;
            let summary =
                run_probe_loop(prober, runner_config, output, quiet, metrics.as_mut()).await?;
            exit_code = exit_code_for_summary(&summary);
        }
        Command::Gtp(command) => {
            let mut metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?;
            let output = make_output(timestamp, timestamp_format.clone(), json, false);
            let (variant, args) = match command.command {
                cli::GtpSubcommand::V1u(args) => (GtpVariant::V1u, args),
                cli::GtpSubcommand::V1c(args) => (GtpVariant::V1c, args),
                cli::GtpSubcommand::V2c(args) => (GtpVariant::V2c, args),
            };
            let quiet = args.quiet;
            let runner_config = RunnerConfig {
                interval: args.interval,
                count: args.count,
                deadline: args.deadline,
            };
            let prober = GtpProber::new(variant, args.target, args.port, args.timeout)
                .await
                .context("failed to initialize GTP prober")?;
            let summary =
                run_probe_loop(prober, runner_config, output, quiet, metrics.as_mut()).await?;
            exit_code = exit_code_for_summary(&summary);
        }
    }

    Ok(exit_code)
}

fn make_output(
    timestamp: TimestampKind,
    timestamp_format: Option<String>,
    json: bool,
    force_timestamp: bool,
) -> Output {
    let timestamp =
        if force_timestamp && timestamp == TimestampKind::None && timestamp_format.is_none() {
            TimestampKind::Local
        } else {
            timestamp
        };
    Output::new(TimestampFormatter::new(timestamp, timestamp_format), json)
}

fn exit_code_for_summary(summary: &Summary) -> ExitCode {
    if summary.sent > 0 && summary.received == 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn exit_code_fails_when_every_probe_is_lost() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 2;
        summary.received = 0;

        assert_eq!(exit_code_for_summary(&summary), ExitCode::FAILURE);
    }

    #[test]
    fn exit_code_succeeds_when_any_probe_replies() {
        let mut summary = Summary::new("target".to_string());
        summary.sent = 2;
        summary.received = 1;
        summary.rtts.push(Duration::from_millis(1));

        assert_eq!(exit_code_for_summary(&summary), ExitCode::SUCCESS);
    }
}
