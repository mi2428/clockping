use std::io::{self, Write};

pub(super) fn write_stdout_line(line: impl AsRef<str>) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", line.as_ref())?;
    Ok(())
}

pub(super) fn write_stderr_line(line: impl AsRef<str>) -> anyhow::Result<()> {
    let mut stderr = io::stderr().lock();
    writeln!(stderr, "{}", line.as_ref())?;
    Ok(())
}

pub(super) fn write_stdout_block(block: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(block.as_bytes())?;
    Ok(())
}

pub fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broken_pipe_errors_are_detected() {
        let error = anyhow::Error::from(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "stdout closed",
        ));

        assert!(is_broken_pipe(&error));
    }
}
