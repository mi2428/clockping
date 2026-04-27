mod cli;
mod event;
mod output;
mod protocol;
mod runner;
mod timefmt;

use anyhow::Context;
use clap::Parser;

use crate::{
    cli::{Cli, Command},
    output::Output,
    protocol::{
        gtp::{GtpProber, GtpVariant},
        icmp::{self, IcmpEngine},
        tcp::TcpProber,
    },
    runner::{RunnerConfig, run_probe_loop},
    timefmt::{TimestampFormatter, TimestampKind},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let timestamp = cli.timestamp;
    let timestamp_format = cli.timestamp_format;
    let json = cli.json;

    match cli.command {
        Command::Icmp(command) => match icmp::parse_engine(command.args)? {
            IcmpEngine::External(external) => {
                let output = make_output(timestamp, timestamp_format.clone(), json, false);
                icmp::run_external(external, output).await?;
            }
            IcmpEngine::Native(config) => {
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
                run_probe_loop(prober, runner_config, output, quiet).await?;
            }
        },
        Command::Tcp(command) => {
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
            run_probe_loop(prober, runner_config, output, quiet).await?;
        }
        Command::Gtp(command) => {
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
            run_probe_loop(prober, runner_config, output, quiet).await?;
        }
    }

    Ok(())
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
