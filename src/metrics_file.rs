use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::{
    metrics::{ProbeMetrics, WindowMetrics},
    prometheus::PrometheusEncoder,
};

const JSONL_SCHEMA_VERSION: u32 = 1;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsFileFormat {
    Jsonl,
    Prometheus,
}

impl MetricsFileFormat {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().as_bytes() {
            b"jsonl" => Some(Self::Jsonl),
            b"prometheus" => Some(Self::Prometheus),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsFileSink {
    path: PathBuf,
    format: MetricsFileFormat,
    encoder: PrometheusEncoder,
}

impl MetricsFileSink {
    pub fn with_prefix_and_labels<I, K, V>(
        path: impl Into<PathBuf>,
        format: MetricsFileFormat,
        metric_prefix: impl Into<String>,
        labels: I,
    ) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let sink = Self {
            path: path.into(),
            format,
            encoder: PrometheusEncoder::with_labels(metric_prefix, labels)?,
        };
        sink.create_empty_file()?;
        Ok(sink)
    }

    pub fn write_interval(&self, metrics: &ProbeMetrics) -> anyhow::Result<()> {
        match self.format {
            MetricsFileFormat::Jsonl => self.append_jsonl("interval", metrics),
            MetricsFileFormat::Prometheus => {
                atomic_write(&self.path, self.encoder.encode_interval(metrics).as_bytes())
            }
        }
    }

    #[allow(dead_code)]
    pub fn write_window(&self, metrics: &WindowMetrics) -> anyhow::Result<()> {
        match self.format {
            MetricsFileFormat::Jsonl => self.append_jsonl("window", metrics),
            MetricsFileFormat::Prometheus => {
                atomic_write(&self.path, self.encoder.encode_window(metrics).as_bytes())
            }
        }
    }

    fn create_empty_file(&self) -> anyhow::Result<()> {
        File::create(&self.path)
            .map(|_| ())
            .map_err(|error| file_error("failed to create metrics file", &self.path, error))
    }

    fn append_jsonl<T>(&self, event: &'static str, metrics: &T) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|error| file_error("failed to open metrics file", &self.path, error))?;
        serde_json::to_writer(
            &mut file,
            &JsonlEvent {
                schema_version: JSONL_SCHEMA_VERSION,
                event,
                metrics,
            },
        )
        .map_err(|error| anyhow::anyhow!("failed to encode metrics JSON: {error}"))?;
        file.write_all(b"\n")
            .map_err(|error| file_error("failed to write metrics file", &self.path, error))
    }
}

#[derive(Serialize)]
struct JsonlEvent<'a, T> {
    schema_version: u32,
    event: &'static str,
    #[serde(flatten)]
    metrics: &'a T,
}

fn file_error(
    message: &'static str,
    path: &Path,
    source: impl std::error::Error + Send + Sync + 'static,
) -> anyhow::Error {
    anyhow::Error::new(source).context(format!("{message}: {}", path.display()))
}

fn atomic_write(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    let temp_path = temp_path_for(path);
    let result = write_temp_then_rename(&temp_path, path, contents);
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result.map_err(|error| file_error("failed to write metrics file", path, error))
}

fn write_temp_then_rename(temp_path: &Path, path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)?;
    file.write_all(contents)?;
    file.flush()?;
    drop(file);
    fs::rename(temp_path, path)
}

fn temp_path_for(path: &Path) -> PathBuf {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "metrics".into());
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(".{file_name}.tmp-{}-{counter}", std::process::id()))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn jsonl_format_appends_interval_events() {
        let path = temp_path("jsonl");
        let sink = MetricsFileSink::with_prefix_and_labels(
            &path,
            MetricsFileFormat::Jsonl,
            "clockping",
            std::iter::empty::<(String, String)>(),
        )
        .unwrap();

        sink.write_interval(&sample_metrics(0)).unwrap();
        sink.write_interval(&sample_metrics(1)).unwrap();

        let output = fs::read_to_string(&path).unwrap();
        let lines = output.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(r#""schema_version":1"#));
        assert!(lines[0].contains(r#""event":"interval""#));
        assert!(lines[1].contains(r#""seq":1"#));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn prometheus_format_replaces_snapshot() {
        let path = temp_path("prom");
        let sink = MetricsFileSink::with_prefix_and_labels(
            &path,
            MetricsFileFormat::Prometheus,
            "nettest",
            [("site", "ci")],
        )
        .unwrap();

        sink.write_interval(&sample_metrics(0)).unwrap();
        sink.write_interval(&sample_metrics(1)).unwrap();

        let output = fs::read_to_string(&path).unwrap();
        assert!(output.contains("nettest_probe_sent{site=\"ci\",protocol=\"tcp\""));
        assert!(output.contains("nettest_probe_sequence"));
        assert!(!output.contains(r#""event":"interval""#));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn format_parser_matches_documented_values() {
        assert_eq!(
            MetricsFileFormat::parse("jsonl"),
            Some(MetricsFileFormat::Jsonl)
        );
        assert_eq!(
            MetricsFileFormat::parse("prometheus"),
            Some(MetricsFileFormat::Prometheus)
        );
        assert_eq!(MetricsFileFormat::parse("xml"), None);
    }

    fn sample_metrics(seq: u64) -> ProbeMetrics {
        ProbeMetrics {
            timestamp_unix_seconds: 1.0,
            protocol: "tcp".to_owned(),
            target: "example:443".to_owned(),
            seq,
            status: "reply",
            sent: seq + 1,
            received: seq + 1,
            lost: 0,
            loss_pct: 0.0,
            up: 1.0,
            rtt_seconds: Some(0.001),
            bytes: None,
            ttl: None,
        }
    }

    fn temp_path(extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "clockping-metrics-file-{}-{nonce}-{counter}.{extension}",
            std::process::id()
        ))
    }
}
