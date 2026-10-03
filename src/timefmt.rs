use chrono::{DateTime, Local};
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum TimestampKind {
    Local,
    Rfc3339,
    Unix,
    UnixMs,
    None,
}

#[derive(Clone, Debug)]
pub struct TimestampFormatter {
    kind: TimestampKind,
    format: Option<String>,
}

impl TimestampFormatter {
    pub fn validate_custom_format(format: &str) -> anyhow::Result<()> {
        if Local::now()
            .format(format)
            .write_to(&mut String::new())
            .is_err()
        {
            anyhow::bail!("invalid --ts.format: {format:?}");
        }
        Ok(())
    }

    pub fn new(kind: TimestampKind, format: Option<String>) -> Self {
        Self { kind, format }
    }

    pub fn format(&self, ts: DateTime<Local>) -> Option<String> {
        if let Some(format) = &self.format {
            return Some(ts.format(format).to_string());
        }

        match self.kind {
            TimestampKind::Local => Some(ts.format("%Y-%m-%d %H:%M:%S%.3f %z").to_string()),
            TimestampKind::Rfc3339 => Some(ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
            TimestampKind::Unix => Some(ts.timestamp().to_string()),
            TimestampKind::UnixMs => Some(ts.timestamp_millis().to_string()),
            TimestampKind::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};

    use super::*;

    #[test]
    fn custom_format_overrides_kind() {
        let formatter =
            TimestampFormatter::new(TimestampKind::None, Some("%Y/%m/%d %H:%M".to_string()));
        let ts = Local.with_ymd_and_hms(2026, 4, 25, 12, 34, 56).unwrap();
        assert_eq!(formatter.format(ts).unwrap(), "2026/04/25 12:34");
    }

    #[test]
    fn none_suppresses_timestamp() {
        let formatter = TimestampFormatter::new(TimestampKind::None, None);
        let ts = Local.with_ymd_and_hms(2026, 4, 25, 12, 34, 56).unwrap();
        assert_eq!(formatter.format(ts), None);
    }

    #[test]
    fn custom_format_validation_rejects_invalid_directives() {
        for format in ["%Q", "%", "%#z", "%Y/%m/%d %H:%M", "literal %% text"] {
            assert_eq!(
                TimestampFormatter::validate_custom_format(format).is_ok(),
                matches!(format, "%Y/%m/%d %H:%M" | "literal %% text"),
                "{format}"
            );
        }
        // Internal format items include both valid fractions and parse-only offsets.
        for format in ["", "%3f %6f %9f", "%z %:z %::z %:::z %Z", "%+ %c %s"] {
            TimestampFormatter::validate_custom_format(format).unwrap();
        }
    }
}
