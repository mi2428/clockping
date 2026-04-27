use std::time::Duration;

use reqwest::{Client, StatusCode, Url};

use crate::{
    metrics::{ProbeMetrics, WindowMetrics},
    prometheus::{render_interval_prometheus, render_window_prometheus, validate_metric_prefix},
};

const PUSH_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);
const PUSH_RETRY_MAX_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct PushGatewayConfig {
    pub endpoint: Url,
    pub job: String,
    pub labels: Vec<(String, String)>,
    pub timeout: Duration,
    pub retries: u32,
    pub user_agent: String,
    pub metric_prefix: String,
    pub delete_on_finish: bool,
}

impl PushGatewayConfig {
    pub const DEFAULT_JOB: &'static str = "clockping";
    pub const DEFAULT_METRIC_PREFIX: &'static str =
        crate::prometheus::PrometheusEncoder::DEFAULT_PREFIX;
    pub const DEFAULT_RETRIES: u32 = 0;
    pub const MAX_RETRIES: u32 = 10;

    pub fn parse_endpoint(raw: &str) -> anyhow::Result<Url> {
        let raw = raw.trim();
        if raw.is_empty() {
            anyhow::bail!("Pushgateway endpoint must not be empty");
        }
        let with_scheme = if raw.starts_with("http://") || raw.starts_with("https://") {
            raw.to_owned()
        } else {
            format!("http://{raw}")
        };
        Url::parse(&with_scheme)
            .map_err(|error| anyhow::anyhow!("invalid Pushgateway endpoint URL: {error}"))
    }

    pub const fn default_timeout() -> Duration {
        Duration::from_secs(5)
    }

    pub fn default_user_agent() -> String {
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        validate_endpoint(&self.endpoint)?;
        validate_job(&self.job)?;
        validate_labels(&self.labels)?;
        validate_timeout(self.timeout)?;
        validate_retries(self.retries)?;
        validate_user_agent(&self.user_agent)?;
        validate_metric_prefix(&self.metric_prefix)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PushGateway {
    client: Client,
    url: Url,
    retries: u32,
    metric_prefix: String,
    delete_on_finish: bool,
}

impl PushGateway {
    pub fn new(config: PushGatewayConfig) -> anyhow::Result<Self> {
        config.validate()?;

        let mut url = config.endpoint;
        let mut path = url.path().trim_end_matches('/').to_owned();
        path.push_str("/metrics/job/");
        path.push_str(&encode_path_segment(&config.job));
        for (name, value) in config.labels {
            path.push('/');
            path.push_str(&encode_path_segment(&name));
            path.push('/');
            path.push_str(&encode_path_segment(&value));
        }
        url.set_path(&path);

        let client = Client::builder()
            .timeout(config.timeout)
            .user_agent(config.user_agent)
            .build()
            .map_err(|error| anyhow::anyhow!("failed to build Pushgateway HTTP client: {error}"))?;

        Ok(Self {
            client,
            url,
            retries: config.retries,
            metric_prefix: config.metric_prefix,
            delete_on_finish: config.delete_on_finish,
        })
    }

    pub async fn push(&self, metrics: &ProbeMetrics) -> anyhow::Result<()> {
        let body = render_interval_prometheus(metrics, &self.metric_prefix);
        self.push_body(&body).await
    }

    pub async fn push_window(&self, metrics: &WindowMetrics) -> anyhow::Result<()> {
        let body = render_window_prometheus(metrics, &self.metric_prefix);
        self.push_body(&body).await
    }

    pub async fn delete(&self) -> anyhow::Result<()> {
        for attempt in 0..=self.retries {
            match self.delete_once().await {
                Ok(()) => return Ok(()),
                Err(error) if error.retryable && attempt < self.retries => {
                    tokio::time::sleep(retry_delay(attempt)).await;
                }
                Err(error) => return Err(error.error),
            }
        }
        unreachable!("delete retry loop always returns")
    }

    pub fn delete_on_finish(&self) -> bool {
        self.delete_on_finish
    }

    #[cfg(test)]
    fn url(&self) -> &Url {
        &self.url
    }

    async fn push_body(&self, body: &str) -> anyhow::Result<()> {
        for attempt in 0..=self.retries {
            match self.push_once(body).await {
                Ok(()) => return Ok(()),
                Err(error) if error.retryable && attempt < self.retries => {
                    tokio::time::sleep(retry_delay(attempt)).await;
                }
                Err(error) => return Err(error.error),
            }
        }
        unreachable!("push retry loop always returns")
    }

    async fn push_once(&self, body: &str) -> Result<(), PushAttemptError> {
        let response = self
            .client
            .put(self.url.clone())
            .header("content-type", "text/plain; version=0.0.4; charset=utf-8")
            .body(body.to_owned())
            .send()
            .await
            .map_err(|error| PushAttemptError {
                error: anyhow::anyhow!("failed to send Pushgateway request: {error}"),
                retryable: true,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(PushAttemptError {
                error: anyhow::anyhow!("Pushgateway returned {status}"),
                retryable: is_retryable_status(status),
            });
        }
        Ok(())
    }

    async fn delete_once(&self) -> Result<(), PushAttemptError> {
        let response = self
            .client
            .delete(self.url.clone())
            .send()
            .await
            .map_err(|error| PushAttemptError {
                error: anyhow::anyhow!("failed to send Pushgateway delete request: {error}"),
                retryable: true,
            })?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(PushAttemptError {
                error: anyhow::anyhow!("Pushgateway delete returned {status}"),
                retryable: is_retryable_status(status),
            });
        }
        Ok(())
    }
}

fn validate_endpoint(endpoint: &Url) -> anyhow::Result<()> {
    match endpoint.scheme() {
        "http" | "https" => {}
        scheme => {
            anyhow::bail!("Pushgateway endpoint scheme must be http or https, got '{scheme}'")
        }
    }
    if endpoint.host_str().is_none() {
        anyhow::bail!("Pushgateway endpoint must include a host");
    }
    Ok(())
}

fn validate_job(job: &str) -> anyhow::Result<()> {
    if job.is_empty() {
        anyhow::bail!("Pushgateway job name must not be empty");
    }
    Ok(())
}

fn validate_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (name, value) in labels {
        validate_label(name, value)?;
    }
    reject_duplicate_labels(labels)
}

pub(crate) fn validate_label(name: &str, value: &str) -> anyhow::Result<()> {
    if !is_valid_label_name(name) {
        anyhow::bail!("invalid Pushgateway label name '{name}'");
    }
    if is_reserved_label_name(name) {
        anyhow::bail!("Pushgateway label name '{name}' is reserved");
    }
    if value.is_empty() {
        anyhow::bail!("Pushgateway label value for '{name}' must not be empty");
    }
    Ok(())
}

fn reject_duplicate_labels(labels: &[(String, String)]) -> anyhow::Result<()> {
    for (index, (name, _)) in labels.iter().enumerate() {
        if labels[..index]
            .iter()
            .any(|(previous_name, _)| previous_name == name)
        {
            anyhow::bail!("duplicate Pushgateway label name '{name}'");
        }
    }
    Ok(())
}

fn validate_timeout(timeout: Duration) -> anyhow::Result<()> {
    if timeout.is_zero() {
        anyhow::bail!("Pushgateway timeout must be greater than zero");
    }
    Ok(())
}

pub(crate) fn validate_retries(retries: u32) -> anyhow::Result<()> {
    if retries > PushGatewayConfig::MAX_RETRIES {
        anyhow::bail!(
            "Pushgateway retries must be at most {}",
            PushGatewayConfig::MAX_RETRIES
        );
    }
    Ok(())
}

pub(crate) fn validate_user_agent(value: &str) -> anyhow::Result<()> {
    if value.is_empty() {
        anyhow::bail!("Pushgateway User-Agent must not be empty");
    }
    if value.chars().any(char::is_control) {
        anyhow::bail!("Pushgateway User-Agent must not contain control characters");
    }
    Ok(())
}

pub(crate) fn is_valid_label_name(name: &str) -> bool {
    let Some((&first, rest)) = name.as_bytes().split_first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    rest.iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
}

pub(crate) fn is_reserved_label_name(name: &str) -> bool {
    name == "job"
}

#[derive(Debug)]
struct PushAttemptError {
    error: anyhow::Error,
    retryable: bool,
}

fn is_retryable_status(status: StatusCode) -> bool {
    let status = status.as_u16();
    status == 429 || (500..=599).contains(&status)
}

fn retry_delay(attempt: u32) -> Duration {
    PUSH_RETRY_BASE_DELAY
        .saturating_mul(2_u32.saturating_pow(attempt))
        .min(PUSH_RETRY_MAX_DELAY)
}

fn encode_path_segment(raw: &str) -> String {
    let mut encoded = String::new();
    for byte in raw.bytes() {
        let encoded_byte = encode_path_byte(byte);
        for &byte in &encoded_byte.bytes[..encoded_byte.len] {
            encoded.push(byte as char);
        }
    }
    encoded
}

#[derive(Debug, Clone, Copy)]
struct EncodedPathByte {
    bytes: [u8; 3],
    len: usize,
}

fn encode_path_byte(byte: u8) -> EncodedPathByte {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    if is_unreserved_path_byte(byte) {
        return EncodedPathByte {
            bytes: [byte, 0, 0],
            len: 1,
        };
    }

    EncodedPathByte {
        bytes: [b'%', HEX[(byte >> 4) as usize], HEX[(byte & 0x0f) as usize]],
        len: 3,
    }
}

fn is_unreserved_path_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_pushgateway_grouping_url() {
        let config = PushGatewayConfig {
            endpoint: Url::parse("http://127.0.0.1:9091/base/").unwrap(),
            job: "clock job".to_owned(),
            labels: vec![
                ("scenario".to_owned(), "sample#1".to_owned()),
                ("site".to_owned(), "tokyo/test".to_owned()),
            ],
            timeout: Duration::from_secs(1),
            retries: 0,
            user_agent: "clockping/test".to_owned(),
            metric_prefix: "clockping".to_owned(),
            delete_on_finish: false,
        };
        let gateway = PushGateway::new(config).unwrap();

        assert_eq!(
            gateway.url().as_str(),
            "http://127.0.0.1:9091/base/metrics/job/clock%20job/scenario/sample%231/site/tokyo%2Ftest"
        );
    }

    #[test]
    fn endpoint_defaults_bare_host_to_http() {
        let url = PushGatewayConfig::parse_endpoint("localhost:9091").unwrap();

        assert_eq!(url.as_str(), "http://localhost:9091/");
    }

    #[test]
    fn rejects_invalid_labels() {
        for (name, value) in [("9bad", "value"), ("job", "value"), ("ok", "")] {
            assert!(validate_label(name, value).is_err());
        }
    }

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay(0), Duration::from_millis(100));
        assert_eq!(retry_delay(10), Duration::from_secs(1));
    }
}
