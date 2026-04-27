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
use tokio::task::JoinSet;

use crate::{
    cli::{Cli, Command},
    metrics::SharedMetricsReporter,
    metrics_options::extract_metrics_options,
    output::Output,
    protocol::{
        gtp::{GtpProber, GtpVariant},
        http::{HttpProber, HttpProberConfig},
        icmp::{self, IcmpEngine},
        tcp::TcpProber,
    },
    runner::{Prober, RunnerConfig, Summary, run_probe_loop},
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
    let colored = cli.colored;
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
                let output = make_output(timestamp, timestamp_format.clone(), json, colored, false);
                icmp::run_external(external, output).await?;
            }
            IcmpEngine::Native(config) => {
                let metrics = metrics_options
                    .take()
                    .expect("metrics options should be consumed once")
                    .into_reporter()?
                    .map(|reporter| reporter.shared());
                let quiet = config.quiet;
                let output = make_output(
                    timestamp,
                    timestamp_format.clone(),
                    json,
                    colored,
                    config.timestamp,
                );
                let runner_config = RunnerConfig {
                    interval: config.interval,
                    count: config.count,
                    deadline: config.deadline,
                };
                let mut probers = Vec::new();
                for destination in config.destinations.clone() {
                    let mut target_config = config.clone();
                    target_config.destinations = vec![destination];
                    probers.push(
                        icmp::NativeIcmpProber::new(target_config)
                            .await
                            .context("failed to initialize native ICMP prober")?,
                    );
                }
                let summaries =
                    run_probers(probers, runner_config, output, quiet, metrics.clone()).await?;
                finish_metrics(metrics).await;
                exit_code = exit_code_for_summaries(&summaries);
            }
        },
        Command::Tcp(command) => {
            let metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?
                .map(|reporter| reporter.shared());
            let quiet = command.quiet;
            let output = make_output(timestamp, timestamp_format.clone(), json, colored, false);
            let runner_config = RunnerConfig {
                interval: command.interval,
                count: command.count,
                deadline: command.deadline,
            };
            let mut probers = Vec::new();
            for target in command.targets {
                probers.push(
                    TcpProber::new(target, command.timeout)
                        .await
                        .context("failed to initialize TCP prober")?,
                );
            }
            let summaries =
                run_probers(probers, runner_config, output, quiet, metrics.clone()).await?;
            finish_metrics(metrics).await;
            exit_code = exit_code_for_summaries(&summaries);
        }
        Command::Http(command) => {
            let metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?
                .map(|reporter| reporter.shared());
            let quiet = command.quiet;
            let output = make_output(timestamp, timestamp_format.clone(), json, colored, false);
            let runner_config = RunnerConfig {
                interval: command.interval,
                count: command.count,
                deadline: command.deadline,
            };
            let method = match command.method {
                cli::HttpMethod::Head => reqwest::Method::HEAD,
                cli::HttpMethod::Get => reqwest::Method::GET,
            };
            let headers = command
                .headers
                .into_iter()
                .map(|header| (header.name, header.value))
                .collect::<Vec<_>>();
            let ok_statuses = command.ok_status.into_ranges();
            let mut probers = Vec::new();
            for target in command.targets {
                probers.push(
                    HttpProber::new(HttpProberConfig {
                        target,
                        method: method.clone(),
                        timeout: command.timeout,
                        headers: headers.clone(),
                        follow_redirects: command.follow_redirects,
                        insecure: command.insecure,
                        ok_statuses: ok_statuses.clone(),
                    })
                    .context("failed to initialize HTTP prober")?,
                );
            }
            let summaries =
                run_probers(probers, runner_config, output, quiet, metrics.clone()).await?;
            finish_metrics(metrics).await;
            exit_code = exit_code_for_summaries(&summaries);
        }
        Command::Gtp(command) => {
            let metrics = metrics_options
                .take()
                .expect("metrics options should be consumed once")
                .into_reporter()?
                .map(|reporter| reporter.shared());
            let output = make_output(timestamp, timestamp_format.clone(), json, colored, false);
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
            let mut probers = Vec::new();
            for target in args.targets {
                probers.push(
                    GtpProber::new(variant, target, args.port, args.timeout)
                        .await
                        .context("failed to initialize GTP prober")?,
                );
            }
            let summaries =
                run_probers(probers, runner_config, output, quiet, metrics.clone()).await?;
            finish_metrics(metrics).await;
            exit_code = exit_code_for_summaries(&summaries);
        }
    }

    Ok(exit_code)
}

async fn run_probers<P>(
    probers: Vec<P>,
    config: RunnerConfig,
    output: Output,
    quiet: bool,
    metrics: Option<SharedMetricsReporter>,
) -> anyhow::Result<Vec<Summary>>
where
    P: Prober + Send + 'static,
{
    let mut tasks = JoinSet::new();
    for prober in probers {
        tasks.spawn(run_probe_loop(
            prober,
            config,
            output.clone(),
            quiet,
            metrics.clone(),
        ));
    }

    let mut summaries = Vec::new();
    while let Some(result) = tasks.join_next().await {
        summaries.push(result??);
    }
    Ok(summaries)
}

async fn finish_metrics(metrics: Option<SharedMetricsReporter>) {
    if let Some(metrics) = metrics {
        metrics.lock().await.finish().await;
    }
}

fn make_output(
    timestamp: TimestampKind,
    timestamp_format: Option<String>,
    json: bool,
    colored: bool,
    force_timestamp: bool,
) -> Output {
    let timestamp =
        if force_timestamp && timestamp == TimestampKind::None && timestamp_format.is_none() {
            TimestampKind::Local
        } else {
            timestamp
        };
    Output::new(
        TimestampFormatter::new(timestamp, timestamp_format),
        json,
        colored,
    )
}

fn exit_code_for_summary(summary: &Summary) -> ExitCode {
    if summary.sent > 0 && summary.received == 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn exit_code_for_summaries(summaries: &[Summary]) -> ExitCode {
    if summaries
        .iter()
        .any(|summary| exit_code_for_summary(summary) == ExitCode::FAILURE)
    {
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

    #[test]
    fn exit_code_fails_when_any_target_loses_every_probe() {
        let mut healthy = Summary::new("healthy".to_string());
        healthy.sent = 2;
        healthy.received = 2;

        let mut down = Summary::new("down".to_string());
        down.sent = 2;
        down.received = 0;

        assert_eq!(exit_code_for_summaries(&[healthy, down]), ExitCode::FAILURE);
    }
}
