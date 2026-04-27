use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub(super) enum AnsiStyle {
    Bold,
    Dim,
    Blue,
    Cyan,
    Green,
    Yellow,
    Magenta,
    Red,
}

impl AnsiStyle {
    const fn code(self) -> &'static str {
        match self {
            Self::Bold => "1",
            Self::Dim => "2",
            Self::Blue => "34",
            Self::Cyan => "36",
            Self::Green => "32",
            Self::Yellow => "33",
            Self::Magenta => "35",
            Self::Red => "31",
        }
    }
}

pub(super) fn paint(enabled: bool, style: AnsiStyle, text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    if enabled {
        format!("\x1b[{}m{text}\x1b[0m", style.code())
    } else {
        text.to_string()
    }
}

pub(super) fn detail_value_style(key: &str, value: &str) -> AnsiStyle {
    match key {
        "icmp_seq" | "gtp_seq" => AnsiStyle::Yellow,
        "ttl" => AnsiStyle::Magenta,
        "status" if value.starts_with('2') || value.starts_with('3') => AnsiStyle::Green,
        "status" => AnsiStyle::Red,
        "method" | "version" | "url" => AnsiStyle::Cyan,
        _ => AnsiStyle::Blue,
    }
}

pub(super) fn loss_count_style(lost: u64) -> AnsiStyle {
    if lost == 0 {
        AnsiStyle::Green
    } else {
        AnsiStyle::Red
    }
}

pub(super) fn loss_percent_style(loss_pct: f64) -> AnsiStyle {
    if loss_pct == 0.0 {
        AnsiStyle::Green
    } else if loss_pct < 100.0 {
        AnsiStyle::Yellow
    } else {
        AnsiStyle::Red
    }
}

pub fn format_duration_ms(duration: Duration) -> String {
    format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
}

pub fn format_duration(duration: Duration) -> String {
    if duration.as_secs() == 0 {
        return format_duration_ms(duration);
    }
    format!("{:.3}s", duration.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_subsecond_duration_as_ms() {
        assert_eq!(format_duration(Duration::from_millis(250)), "250.000ms");
    }

    #[test]
    fn format_second_duration_as_seconds() {
        assert_eq!(format_duration(Duration::from_millis(1250)), "1.250s");
    }
}
