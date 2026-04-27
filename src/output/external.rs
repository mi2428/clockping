use chrono::Local;

use super::{
    Output, json,
    style::AnsiStyle,
    writer::{write_stderr_line, write_stdout_line},
};

pub(super) fn print_line(output: &Output, stream: &'static str, line: &str) -> anyhow::Result<()> {
    let ts = Local::now();
    if output.is_json() {
        let timestamp = output.timestamp(ts).unwrap_or_default();
        return json::print_external_line(output, stream, line, timestamp);
    }

    match output.timestamp(ts) {
        Some(timestamp) => write_stdout_line(format!(
            "{} {line}",
            output.paint(AnsiStyle::Dim, timestamp)
        ))?,
        None => write_stdout_line(line)?,
    }
    Ok(())
}

pub(super) fn print_line_without_timestamp(
    output: &Output,
    stream: &'static str,
    line: &str,
) -> anyhow::Result<()> {
    if output.is_json() {
        return json::print_external_line(output, stream, line, String::new());
    }

    write_stdout_line(line)
}

pub(super) fn print_stderr_line(line: &str) -> anyhow::Result<()> {
    write_stderr_line(line)
}
