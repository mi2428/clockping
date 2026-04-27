use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use reqwest::Url;

use crate::{
    metrics::{MetricsReporter, PushGatewaySink},
    metrics_file::{MetricsFileFormat, MetricsFileSink},
    prometheus::{is_valid_label_name, reject_duplicate_labels, validate_metric_prefix},
    pushgateway::{PushGateway, PushGatewayConfig, validate_retries, validate_user_agent},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DurationUnit {
    Milliseconds,
    Seconds,
    Minutes,
}

#[derive(Debug)]
pub struct MetricsOptions {
    pub push_url: Option<Url>,
    pub push_job: String,
    pub push_labels: Vec<(String, String)>,
    pub push_timeout: Duration,
    pub push_retries: u32,
    pub push_user_agent: String,
    pub metrics_prefix: String,
    pub push_interval: Option<Duration>,
    pub push_delete_on_exit: bool,
    pub metrics_file: Option<PathBuf>,
    pub metrics_format: MetricsFileFormat,
    pub metrics_labels: Vec<(String, String)>,
}

impl MetricsOptions {
    pub fn is_enabled(&self) -> bool {
        self.push_url.is_some() || self.metrics_file.is_some()
    }

    pub fn into_reporter(self) -> anyhow::Result<Option<MetricsReporter>> {
        let pushgateway = self
            .push_url
            .map(|endpoint| {
                PushGateway::new(PushGatewayConfig {
                    endpoint,
                    job: self.push_job,
                    labels: self.push_labels,
                    timeout: self.push_timeout,
                    retries: self.push_retries,
                    user_agent: self.push_user_agent,
                    metric_prefix: self.metrics_prefix.clone(),
                    delete_on_finish: self.push_delete_on_exit,
                })
                .map(|gateway| PushGatewaySink::new(gateway, self.push_interval))
            })
            .transpose()?;

        let file = self
            .metrics_file
            .map(|path| {
                MetricsFileSink::with_prefix_and_labels(
                    path,
                    self.metrics_format,
                    self.metrics_prefix,
                    self.metrics_labels,
                )
            })
            .transpose()?;

        let reporter = MetricsReporter::new(pushgateway, file);
        Ok((!reporter.is_empty()).then_some(reporter))
    }
}

pub fn extract_metrics_options(
    args: Vec<OsString>,
) -> anyhow::Result<(MetricsOptions, Vec<OsString>)> {
    extract_metrics_options_with_env(args, |key| env::var(key).ok())
}

fn extract_metrics_options_with_env(
    args: Vec<OsString>,
    mut get_env: impl FnMut(&str) -> Option<String>,
) -> anyhow::Result<(MetricsOptions, Vec<OsString>)> {
    let mut pass_through = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();
    let program = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing argv[0]"))?;
    pass_through.push(program);

    let rest = iter.collect::<Vec<_>>();
    let informational = find_informational_request(&rest);
    let mut env_lookup = |key: &str| {
        if informational { None } else { get_env(key) }
    };

    let mut push_url = env_value(&mut env_lookup, "CLOCKPING_PUSH_URL", "IPERF3_PUSH_URL");
    let mut push_job = env_value(&mut env_lookup, "CLOCKPING_PUSH_JOB", "IPERF3_PUSH_JOB")
        .unwrap_or_else(|| PushGatewayConfig::DEFAULT_JOB.to_owned());
    let mut push_labels = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_LABELS",
        "IPERF3_PUSH_LABELS",
    )
    .map(|raw| parse_env_labels("CLOCKPING_PUSH_LABELS", &raw, true))
    .transpose()?
    .unwrap_or_default();
    let mut push_timeout = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_TIMEOUT",
        "IPERF3_PUSH_TIMEOUT",
    )
    .map(|raw| parse_duration_option("CLOCKPING_PUSH_TIMEOUT", &raw))
    .transpose()?
    .unwrap_or_else(PushGatewayConfig::default_timeout);
    let mut push_retries = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_RETRIES",
        "IPERF3_PUSH_RETRIES",
    )
    .map(|raw| parse_retries("CLOCKPING_PUSH_RETRIES", &raw))
    .transpose()?
    .unwrap_or(PushGatewayConfig::DEFAULT_RETRIES);
    let mut push_user_agent = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_USER_AGENT",
        "IPERF3_PUSH_USER_AGENT",
    )
    .map(|raw| parse_user_agent("CLOCKPING_PUSH_USER_AGENT", &raw))
    .transpose()?
    .unwrap_or_else(PushGatewayConfig::default_user_agent);
    let mut metrics_prefix = env_value(
        &mut env_lookup,
        "CLOCKPING_METRICS_PREFIX",
        "IPERF3_METRICS_PREFIX",
    )
    .map(|raw| parse_metric_prefix("CLOCKPING_METRICS_PREFIX", &raw))
    .transpose()?
    .unwrap_or_else(|| PushGatewayConfig::DEFAULT_METRIC_PREFIX.to_owned());
    let mut push_interval = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_INTERVAL",
        "IPERF3_PUSH_INTERVAL",
    )
    .map(|raw| parse_duration_option("CLOCKPING_PUSH_INTERVAL", &raw))
    .transpose()?;
    let mut push_delete_on_exit = env_value(
        &mut env_lookup,
        "CLOCKPING_PUSH_DELETE_ON_EXIT",
        "IPERF3_PUSH_DELETE_ON_EXIT",
    )
    .map(|raw| parse_bool_option("CLOCKPING_PUSH_DELETE_ON_EXIT", &raw))
    .transpose()?
    .unwrap_or(false);
    let mut metrics_file = env_value(
        &mut env_lookup,
        "CLOCKPING_METRICS_FILE",
        "IPERF3_METRICS_FILE",
    )
    .map(PathBuf::from);
    let raw_metrics_format = env_value(
        &mut env_lookup,
        "CLOCKPING_METRICS_FORMAT",
        "IPERF3_METRICS_FORMAT",
    );
    let mut metrics_format = raw_metrics_format
        .as_deref()
        .map(|raw| parse_metrics_format("CLOCKPING_METRICS_FORMAT", raw))
        .transpose()?
        .unwrap_or(MetricsFileFormat::Jsonl);
    let mut metrics_labels = env_value(
        &mut env_lookup,
        "CLOCKPING_METRICS_LABELS",
        "IPERF3_METRICS_LABELS",
    )
    .map(|raw| parse_env_labels("CLOCKPING_METRICS_LABELS", &raw, false))
    .transpose()?
    .unwrap_or_default();

    let mut saw_push_job = false;
    let mut saw_push_label = !push_labels.is_empty();
    let mut saw_push_setting = false;
    let mut saw_metrics_setting = raw_metrics_format.is_some();
    let mut saw_metrics_label = !metrics_labels.is_empty();
    let mut saw_metric_prefix = false;

    let mut i = 0;
    while i < rest.len() {
        let arg = &rest[i];
        if arg == "--" {
            pass_through.extend(rest[i..].iter().cloned());
            break;
        }

        let Some(arg_text) = arg.to_str() else {
            pass_through.push(arg.clone());
            i += 1;
            continue;
        };

        if let Some((key, value)) = split_long_value(arg_text) {
            match key {
                "--push.url" => push_url = Some(value.to_owned()),
                "--push.job" => {
                    push_job = value.to_owned();
                    saw_push_job = true;
                }
                "--push.label" => {
                    push_labels.push(parse_label("--push.label", value, true)?);
                    saw_push_label = true;
                }
                "--metrics.label" => {
                    metrics_labels.push(parse_label("--metrics.label", value, false)?);
                    saw_metrics_label = true;
                }
                "--push.timeout" => {
                    push_timeout = parse_duration_option("--push.timeout", value)?;
                    saw_push_setting = true;
                }
                "--push.retries" => {
                    push_retries = parse_retries("--push.retries", value)?;
                    saw_push_setting = true;
                }
                "--push.user-agent" => {
                    push_user_agent = parse_user_agent("--push.user-agent", value)?;
                    saw_push_setting = true;
                }
                "--metrics.prefix" => {
                    metrics_prefix = parse_metric_prefix("--metrics.prefix", value)?;
                    saw_metric_prefix = true;
                }
                "--push.interval" => {
                    push_interval = Some(parse_duration_option("--push.interval", value)?);
                    saw_push_setting = true;
                }
                "--push.delete-on-exit" => {
                    push_delete_on_exit = parse_bool_option("--push.delete-on-exit", value)?;
                    saw_push_setting = true;
                }
                "--metrics.file" => metrics_file = Some(PathBuf::from(value)),
                "--metrics.format" => {
                    metrics_format = parse_metrics_format("--metrics.format", value)?;
                    saw_metrics_setting = true;
                }
                _ => pass_through.push(arg.clone()),
            }
            i += 1;
            continue;
        }

        match arg_text {
            "--push.url" => push_url = Some(take_string_value(&rest, &mut i, "--push.url")?),
            "--push.job" => {
                push_job = take_string_value(&rest, &mut i, "--push.job")?;
                saw_push_job = true;
            }
            "--push.label" => {
                let value = take_string_value(&rest, &mut i, "--push.label")?;
                push_labels.push(parse_label("--push.label", &value, true)?);
                saw_push_label = true;
            }
            "--metrics.label" => {
                let value = take_string_value(&rest, &mut i, "--metrics.label")?;
                metrics_labels.push(parse_label("--metrics.label", &value, false)?);
                saw_metrics_label = true;
            }
            "--push.timeout" => {
                let value = take_string_value(&rest, &mut i, "--push.timeout")?;
                push_timeout = parse_duration_option("--push.timeout", &value)?;
                saw_push_setting = true;
            }
            "--push.retries" => {
                let value = take_string_value(&rest, &mut i, "--push.retries")?;
                push_retries = parse_retries("--push.retries", &value)?;
                saw_push_setting = true;
            }
            "--push.user-agent" => {
                let value = take_string_value(&rest, &mut i, "--push.user-agent")?;
                push_user_agent = parse_user_agent("--push.user-agent", &value)?;
                saw_push_setting = true;
            }
            "--metrics.prefix" => {
                let value = take_string_value(&rest, &mut i, "--metrics.prefix")?;
                metrics_prefix = parse_metric_prefix("--metrics.prefix", &value)?;
                saw_metric_prefix = true;
            }
            "--push.interval" => {
                let value = take_string_value(&rest, &mut i, "--push.interval")?;
                push_interval = Some(parse_duration_option("--push.interval", &value)?);
                saw_push_setting = true;
            }
            "--push.delete-on-exit" => {
                push_delete_on_exit = true;
                saw_push_setting = true;
                i += 1;
            }
            "--metrics.file" => {
                metrics_file = Some(take_path_value(&rest, &mut i, "--metrics.file")?)
            }
            "--metrics.format" => {
                let value = take_string_value(&rest, &mut i, "--metrics.format")?;
                metrics_format = parse_metrics_format("--metrics.format", &value)?;
                saw_metrics_setting = true;
            }
            _ => {
                pass_through.push(arg.clone());
                i += 1;
            }
        }
    }

    let push_url = push_url.as_deref().map(parse_url).transpose()?;
    if !informational {
        validate_option_dependencies(
            push_url.is_some(),
            metrics_file.is_some(),
            saw_push_job,
            saw_push_label,
            saw_push_setting,
            saw_metrics_setting,
            saw_metrics_label,
            saw_metric_prefix,
            metrics_format,
            &push_job,
            &push_labels,
            &metrics_labels,
        )?;
    }

    Ok((
        MetricsOptions {
            push_url,
            push_job,
            push_labels,
            push_timeout,
            push_retries,
            push_user_agent,
            metrics_prefix,
            push_interval,
            push_delete_on_exit,
            metrics_file,
            metrics_format,
            metrics_labels,
        },
        pass_through,
    ))
}

#[allow(clippy::too_many_arguments)]
fn validate_option_dependencies(
    push_enabled: bool,
    file_enabled: bool,
    saw_push_job: bool,
    saw_push_label: bool,
    saw_push_setting: bool,
    saw_metrics_setting: bool,
    saw_metrics_label: bool,
    saw_metric_prefix: bool,
    metrics_format: MetricsFileFormat,
    push_job: &str,
    push_labels: &[(String, String)],
    metrics_labels: &[(String, String)],
) -> anyhow::Result<()> {
    if !push_enabled && saw_push_job {
        anyhow::bail!("--push.job requires --push.url or CLOCKPING_PUSH_URL");
    }
    if !push_enabled && saw_push_label {
        anyhow::bail!("--push.label requires --push.url or CLOCKPING_PUSH_URL");
    }
    if !push_enabled && saw_push_setting {
        anyhow::bail!("push settings require --push.url or CLOCKPING_PUSH_URL");
    }
    if !file_enabled && saw_metrics_setting {
        anyhow::bail!("metrics settings require --metrics.file or CLOCKPING_METRICS_FILE");
    }
    if !file_enabled && saw_metrics_label {
        anyhow::bail!("--metrics.label requires --metrics.file or CLOCKPING_METRICS_FILE");
    }
    if saw_metrics_label && metrics_format != MetricsFileFormat::Prometheus {
        anyhow::bail!("--metrics.label requires --metrics.format prometheus");
    }
    if !push_enabled && !file_enabled && saw_metric_prefix {
        anyhow::bail!(
            "metric prefix requires --metrics.file, CLOCKPING_METRICS_FILE, --push.url, or CLOCKPING_PUSH_URL"
        );
    }
    if push_enabled && push_job.is_empty() {
        anyhow::bail!("--push.job must not be empty when --push.url is set");
    }
    reject_duplicate_push_labels(push_labels)?;
    reject_duplicate_labels(metrics_labels)?;
    reject_dynamic_metric_labels(metrics_labels)?;
    Ok(())
}

fn split_long_value(arg: &str) -> Option<(&str, &str)> {
    arg.split_once('=').filter(|(key, _)| key.starts_with("--"))
}

fn take_string_value(args: &[OsString], index: &mut usize, option: &str) -> anyhow::Result<String> {
    let value = take_value(args, index, option)?;
    value
        .into_string()
        .map_err(|_| anyhow::anyhow!("{option} requires a UTF-8 value"))
}

fn take_path_value(args: &[OsString], index: &mut usize, option: &str) -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from(take_value(args, index, option)?))
}

fn take_value(args: &[OsString], index: &mut usize, option: &str) -> anyhow::Result<OsString> {
    *index += 1;
    let value = args
        .get(*index)
        .ok_or_else(|| anyhow::anyhow!("{option} requires a value"))?;
    *index += 1;
    Ok(value.clone())
}

fn parse_url(raw: &str) -> anyhow::Result<Url> {
    PushGatewayConfig::parse_endpoint(raw)
        .map_err(|error| anyhow::anyhow!("invalid --push.url URL: {error}"))
}

fn parse_duration_option(option: &str, raw: &str) -> anyhow::Result<Duration> {
    let raw = raw.trim();
    if raw.is_empty() {
        anyhow::bail!("{option} must not be empty");
    }

    let duration = if let Some(number) = raw.strip_suffix("ms") {
        duration_from_number(
            parse_duration_number(option, raw, number)?,
            DurationUnit::Milliseconds,
        )
        .expect("millisecond durations cannot overflow")
    } else if let Some(number) = raw.strip_suffix('s') {
        duration_from_number(
            parse_duration_number(option, raw, number)?,
            DurationUnit::Seconds,
        )
        .expect("second durations cannot overflow")
    } else if let Some(number) = raw.strip_suffix('m') {
        duration_from_number(
            parse_duration_number(option, raw, number)?,
            DurationUnit::Minutes,
        )
        .ok_or_else(|| anyhow::anyhow!("{option} is too large: {raw}"))?
    } else {
        duration_from_number(
            parse_duration_number(option, raw, raw)?,
            DurationUnit::Seconds,
        )
        .expect("second durations cannot overflow")
    };

    if duration.is_zero() {
        anyhow::bail!("{option} must be greater than zero");
    }
    Ok(duration)
}

fn parse_duration_number(option: &str, raw: &str, number: &str) -> anyhow::Result<u64> {
    if number.is_empty() {
        anyhow::bail!("invalid {option} duration: {raw}");
    }
    number
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("invalid {option} duration: {raw}"))
}

fn duration_from_number(number: u64, unit: DurationUnit) -> Option<Duration> {
    match unit {
        DurationUnit::Milliseconds => Some(Duration::from_millis(number)),
        DurationUnit::Seconds => Some(Duration::from_secs(number)),
        DurationUnit::Minutes => number.checked_mul(60).map(Duration::from_secs),
    }
}

fn parse_retries(option: &str, raw: &str) -> anyhow::Result<u32> {
    let retries = raw.trim().parse::<u32>().map_err(|_| {
        anyhow::anyhow!(
            "{option} must be an integer between 0 and {}",
            PushGatewayConfig::MAX_RETRIES
        )
    })?;
    validate_retries(retries).map_err(|_| {
        anyhow::anyhow!(
            "{option} must be at most {}",
            PushGatewayConfig::MAX_RETRIES
        )
    })?;
    Ok(retries)
}

fn parse_bool_option(option: &str, raw: &str) -> anyhow::Result<bool> {
    parse_bool_literal(raw.trim()).ok_or_else(|| {
        anyhow::anyhow!("{option} must be one of true, false, 1, 0, yes, no, on, or off")
    })
}

fn parse_bool_literal(raw: &str) -> Option<bool> {
    if ["1", "true", "yes", "on"]
        .iter()
        .any(|value| raw.eq_ignore_ascii_case(value))
    {
        return Some(true);
    }
    if ["0", "false", "no", "off"]
        .iter()
        .any(|value| raw.eq_ignore_ascii_case(value))
    {
        return Some(false);
    }
    None
}

fn parse_metrics_format(option: &str, raw: &str) -> anyhow::Result<MetricsFileFormat> {
    MetricsFileFormat::parse(raw)
        .ok_or_else(|| anyhow::anyhow!("{option} must be one of jsonl or prometheus"))
}

fn parse_user_agent(option: &str, raw: &str) -> anyhow::Result<String> {
    let value = raw.trim();
    validate_user_agent(value).map_err(|error| {
        anyhow::anyhow!(
            "{}",
            error.to_string().replace("Pushgateway User-Agent", option)
        )
    })?;
    Ok(value.to_owned())
}

fn parse_metric_prefix(option: &str, raw: &str) -> anyhow::Result<String> {
    let value = raw.trim();
    validate_metric_prefix(value)
        .map_err(|_| anyhow::anyhow!("invalid {option} metric prefix '{value}'"))?;
    Ok(value.to_owned())
}

fn parse_env_labels(
    option: &str,
    raw: &str,
    reserve_job: bool,
) -> anyhow::Result<Vec<(String, String)>> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    raw.split(',')
        .map(str::trim)
        .map(|label| parse_label(option, label, reserve_job))
        .collect()
}

fn parse_label(option: &str, raw: &str, reserve_job: bool) -> anyhow::Result<(String, String)> {
    let (name, value) = raw
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("{option} requires KEY=VALUE"))?;
    if !is_valid_label_name(name) {
        anyhow::bail!("invalid {option} name '{name}'");
    }
    if reserve_job && crate::pushgateway::is_reserved_label_name(name) {
        anyhow::bail!("{option} name '{name}' is reserved");
    }
    if value.is_empty() {
        anyhow::bail!("{option} value for '{name}' must not be empty");
    }
    Ok((name.to_owned(), value.to_owned()))
}

fn reject_duplicate_push_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (index, (name, _)) in labels.iter().enumerate() {
        if labels[..index]
            .iter()
            .any(|(previous_name, _)| previous_name == name)
        {
            anyhow::bail!("duplicate --push.label name '{name}'");
        }
    }
    Ok(())
}

fn reject_dynamic_metric_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (name, _) in labels {
        if matches!(name.as_str(), "protocol" | "target" | "status") {
            anyhow::bail!("--metrics.label name '{name}' is reserved");
        }
    }
    Ok(())
}

fn env_value(
    get_env: &mut impl FnMut(&str) -> Option<String>,
    primary: &str,
    fallback: &str,
) -> Option<String> {
    get_env(primary).or_else(|| get_env(fallback))
}

fn find_informational_request(args: &[OsString]) -> bool {
    for arg in args {
        if arg == "--" {
            break;
        }
        let Some(text) = arg.to_str() else {
            continue;
        };
        if matches!(text, "-h" | "--help" | "help" | "-V" | "--version") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_metrics_options_from_clockping_argv() {
        let args = vec![
            "clockping".into(),
            "--push.url".into(),
            "localhost:9091".into(),
            "--push.job=net".into(),
            "--push.label".into(),
            "scenario=sample".into(),
            "--push.timeout=2s".into(),
            "--push.retries".into(),
            "2".into(),
            "--push.user-agent=clockping/custom".into(),
            "--metrics.prefix".into(),
            "nettest".into(),
            "--push.interval=10s".into(),
            "--push.delete-on-exit".into(),
            "--metrics.file".into(),
            "metrics.jsonl".into(),
            "--metrics.format=prometheus".into(),
            "--metrics.label".into(),
            "site=ci".into(),
            "tcp".into(),
            "-c".into(),
            "1".into(),
            "127.0.0.1:80".into(),
        ];

        let (options, cli) = extract_metrics_options_with_env(args, |_| None).unwrap();

        assert_eq!(options.push_url.unwrap().as_str(), "http://localhost:9091/");
        assert_eq!(options.push_job, "net");
        assert_eq!(
            options.push_labels,
            [("scenario".to_owned(), "sample".to_owned())]
        );
        assert_eq!(options.push_timeout, Duration::from_secs(2));
        assert_eq!(options.push_retries, 2);
        assert_eq!(options.push_user_agent, "clockping/custom");
        assert_eq!(options.metrics_prefix, "nettest");
        assert_eq!(options.push_interval, Some(Duration::from_secs(10)));
        assert!(options.push_delete_on_exit);
        assert_eq!(options.metrics_file, Some(PathBuf::from("metrics.jsonl")));
        assert_eq!(options.metrics_format, MetricsFileFormat::Prometheus);
        assert_eq!(
            options.metrics_labels,
            [("site".to_owned(), "ci".to_owned())]
        );
        assert_eq!(cli, ["clockping", "tcp", "-c", "1", "127.0.0.1:80"]);
    }

    #[test]
    fn clockping_environment_overrides_iperf3_aliases() {
        let args = vec!["clockping".into(), "tcp".into(), "127.0.0.1:80".into()];

        let (options, cli) = extract_metrics_options_with_env(args, |key| match key {
            "IPERF3_PUSH_URL" => Some("http://iperf.example:9091".to_owned()),
            "CLOCKPING_PUSH_URL" => Some("http://clock.example:9091".to_owned()),
            "IPERF3_METRICS_PREFIX" => Some("iperf_prefix".to_owned()),
            "CLOCKPING_METRICS_PREFIX" => Some("clock_prefix".to_owned()),
            _ => None,
        })
        .unwrap();

        assert_eq!(
            options.push_url.unwrap().as_str(),
            "http://clock.example:9091/"
        );
        assert_eq!(options.metrics_prefix, "clock_prefix");
        assert_eq!(cli, ["clockping", "tcp", "127.0.0.1:80"]);
    }

    #[test]
    fn accepts_iperf3_environment_aliases() {
        let args = vec!["clockping".into(), "tcp".into(), "127.0.0.1:80".into()];

        let (options, _cli) = extract_metrics_options_with_env(args, |key| match key {
            "IPERF3_PUSH_URL" => Some("localhost:9091".to_owned()),
            "IPERF3_PUSH_LABELS" => Some("scenario=alias".to_owned()),
            "IPERF3_METRICS_FILE" => Some("metrics.jsonl".to_owned()),
            _ => None,
        })
        .unwrap();

        assert_eq!(options.push_url.unwrap().as_str(), "http://localhost:9091/");
        assert_eq!(
            options.push_labels,
            [("scenario".to_owned(), "alias".to_owned())]
        );
        assert_eq!(options.metrics_file, Some(PathBuf::from("metrics.jsonl")));
    }

    #[test]
    fn rejects_push_settings_without_push_url() {
        for option in [
            "--push.job=job",
            "--push.label=scenario=test",
            "--push.timeout=5s",
            "--push.retries=1",
            "--push.user-agent=clockping/test",
            "--push.interval=10s",
            "--push.delete-on-exit",
        ] {
            let args = vec!["clockping".into(), option.into(), "tcp".into()];
            assert!(
                extract_metrics_options_with_env(args, |_| None).is_err(),
                "{option}"
            );
        }
    }

    #[test]
    fn rejects_metrics_settings_without_metrics_file() {
        let args = vec![
            "clockping".into(),
            "--metrics.format=prometheus".into(),
            "tcp".into(),
        ];

        let error = extract_metrics_options_with_env(args, |_| None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("metrics settings require --metrics.file")
        );
    }

    #[test]
    fn metrics_labels_require_prometheus_file_output() {
        let args = vec![
            "clockping".into(),
            "--metrics.file=metrics.jsonl".into(),
            "--metrics.label=site=ci".into(),
            "tcp".into(),
        ];

        let error = extract_metrics_options_with_env(args, |_| None).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("--metrics.label requires --metrics.format prometheus")
        );
    }

    #[test]
    fn rejects_malformed_values() {
        for (option, value, expected) in [
            (
                "--push.timeout",
                "0",
                "--push.timeout must be greater than zero",
            ),
            (
                "--push.timeout",
                "1h",
                "invalid --push.timeout duration: 1h",
            ),
            ("--push.retries", "11", "--push.retries must be at most 10"),
            (
                "--push.user-agent",
                "",
                "--push.user-agent must not be empty",
            ),
            (
                "--metrics.prefix",
                "bad-prefix",
                "invalid --metrics.prefix metric prefix",
            ),
            (
                "--push.interval",
                "0",
                "--push.interval must be greater than zero",
            ),
            (
                "--metrics.format",
                "xml",
                "--metrics.format must be one of jsonl or prometheus",
            ),
        ] {
            let args = vec![
                "clockping".into(),
                "--metrics.file".into(),
                "metrics.prom".into(),
                option.into(),
                value.into(),
            ];
            let error = extract_metrics_options_with_env(args, |_| None).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "{option}={value:?} should fail with {expected:?}, got {error:#}"
            );
        }
    }

    #[test]
    fn parses_bool_options() {
        for value in ["1", "true", "yes", "on"] {
            assert!(parse_bool_option("--push.delete-on-exit", value).unwrap());
        }
        for value in ["0", "false", "no", "off"] {
            assert!(!parse_bool_option("--push.delete-on-exit", value).unwrap());
        }
        assert!(parse_bool_option("--push.delete-on-exit", "maybe").is_err());
    }
}
