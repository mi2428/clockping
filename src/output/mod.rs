mod external;
mod json;
mod style;
mod text;
mod writer;

use chrono::{DateTime, Local};

use crate::{event::ProbeEvent, runner::Summary, timefmt::TimestampFormatter};

pub use writer::is_broken_pipe;

use self::{style::AnsiStyle, writer::write_stdout_line};

#[derive(Clone, Debug)]
pub struct Output {
    timestamps: TimestampFormatter,
    json: bool,
    colored: bool,
}

impl Output {
    pub fn new(timestamps: TimestampFormatter, json: bool, colored: bool) -> Self {
        Self {
            timestamps,
            json,
            colored: colored && !json,
        }
    }

    pub fn timestamp(&self, ts: DateTime<Local>) -> Option<String> {
        self.timestamps.format(ts)
    }

    pub fn print_external_line(&self, stream: &'static str, line: &str) -> anyhow::Result<()> {
        external::print_line(self, stream, line)
    }

    pub fn print_external_line_without_timestamp(
        &self,
        stream: &'static str,
        line: &str,
    ) -> anyhow::Result<()> {
        external::print_line_without_timestamp(self, stream, line)
    }

    pub fn print_external_stderr_line(&self, line: &str) -> anyhow::Result<()> {
        external::print_stderr_line(line)
    }

    pub fn print_event(&self, event: &ProbeEvent) -> anyhow::Result<()> {
        if self.json {
            return json::print_event(self, event);
        }

        write_stdout_line(text::build_event(self, event))?;
        Ok(())
    }

    pub fn print_summary(&self, summary: &Summary, quiet: bool) -> anyhow::Result<()> {
        if self.json {
            if quiet {
                json::print_summary(self, summary)?;
            }
            return Ok(());
        }

        writer::write_stdout_block(&text::build_summary(self, summary))?;
        Ok(())
    }

    fn is_json(&self) -> bool {
        self.json
    }

    fn paint(&self, style: AnsiStyle, text: impl AsRef<str>) -> String {
        style::paint(self.colored, style, text)
    }
}
