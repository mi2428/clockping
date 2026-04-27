use std::{
    ffi::OsString,
    path::PathBuf,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::Context;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

use crate::output::Output;

#[derive(Debug, Clone)]
pub struct ExternalPingConfig {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

pub async fn run_external(config: ExternalPingConfig, output: Output) -> anyhow::Result<()> {
    let mut command = Command::new(&config.program);
    command
        .args(&config.args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    configure_external_command(&mut command);

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn {}", config.program.display()))?;

    let stdout = child.stdout.take().context("failed to capture stdout")?;
    let stderr = child.stderr.take().context("failed to capture stderr")?;
    let stdout_output = output.clone();
    let stderr_output = output.clone();
    let interrupted = Arc::new(AtomicBool::new(false));
    let stdout_interrupted = Arc::clone(&interrupted);

    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            if stdout_interrupted.load(Ordering::Acquire) {
                stdout_output.print_external_line_without_timestamp("stdout", &line)?;
            } else {
                stdout_output.print_external_line("stdout", &line)?;
            }
        }
        anyhow::Ok(())
    });

    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Some(line) = lines.next_line().await? {
            stderr_output.print_external_stderr_line(&line)?;
        }
        anyhow::Ok(())
    });

    let (status, interrupted) = wait_for_external_child(&mut child, &interrupted).await?;
    stdout_task.await??;
    stderr_task.await??;
    if !status.success() && !interrupted {
        anyhow::bail!("{} exited with {status}", config.program.display());
    }
    Ok(())
}

async fn wait_for_external_child(
    child: &mut Child,
    interrupted: &AtomicBool,
) -> anyhow::Result<(ExitStatus, bool)> {
    tokio::select! {
        status = child.wait() => Ok((status?, false)),
        interrupt = tokio::signal::ctrl_c() => {
            interrupt.context("failed to listen for Ctrl-C")?;
            interrupted.store(true, Ordering::Release);
            Ok((wait_after_ctrl_c(child).await?, true))
        }
    }
}

async fn wait_after_ctrl_c(child: &mut Child) -> anyhow::Result<ExitStatus> {
    interrupt_external_child(child)?;
    Ok(child.wait().await?)
}

#[cfg(unix)]
fn configure_external_command(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_external_command(_command: &mut Command) {}

#[cfg(unix)]
fn interrupt_external_child(child: &mut Child) -> anyhow::Result<()> {
    let Some(id) = child.id() else {
        return Ok(());
    };
    let process_group = -(id as libc::pid_t);
    // SAFETY: `kill` only receives the child process group derived from the
    // Tokio child PID and does not dereference any pointers.
    let result = unsafe { libc::kill(process_group, libc::SIGINT) };
    if result == 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).context("failed to send SIGINT to external pinger")
}

#[cfg(not(unix))]
fn interrupt_external_child(child: &mut Child) -> anyhow::Result<()> {
    child
        .start_kill()
        .context("failed to stop external pinger after Ctrl-C")
}
