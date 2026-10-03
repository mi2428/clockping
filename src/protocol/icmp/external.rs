use std::{
    ffi::OsString,
    future::Future,
    path::PathBuf,
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::Context;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
};

use crate::output::Output;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct ExternalPingConfig {
    pub program: PathBuf,
    pub args: Vec<OsString>,
}

pub async fn run_external(config: ExternalPingConfig, output: Output) -> anyhow::Result<()> {
    // Register before spawning: child output can reach the fixture immediately.
    #[cfg(unix)]
    let mut signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .context("failed to listen for Ctrl-C")?;
    #[cfg(unix)]
    let interrupt = async { signal.recv().await.context("Ctrl-C listener closed") };
    #[cfg(not(unix))]
    let interrupt = async {
        tokio::signal::ctrl_c()
            .await
            .context("failed to listen for Ctrl-C")
    };

    let mut command = Command::new(&config.program);
    command
        .args(&config.args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    configure_external_command(&mut command);

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn {}", config.program.display()))?;
    // Keep the owned group ID even if the leader exits before its pipes close.
    let id = child.id().context("spawned external pinger has no PID")?;

    let result = async {
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

        let stdout_abort = stdout_task.abort_handle();
        let stderr_abort = stderr_task.abort_handle();
        let drained = async {
            tokio::try_join!(async { stdout_task.await? }, async { stderr_task.await? })?;
            anyhow::Ok(())
        };
        tokio::pin!(drained);
        let result =
            wait_for_external_child(&mut child, id, &interrupted, interrupt, &mut drained).await;
        stdout_abort.abort();
        stderr_abort.abort();
        let (status, interrupted) = result?;
        if !status.success() && !interrupted {
            anyhow::bail!("{} exited with {status}", config.program.display());
        }
        anyhow::Ok(())
    }
    .await;

    if let Err(error) = result {
        // An I/O error can occur before the leader exits. Its unreaped PID still
        // owns the group; never signal a group after wait() released that PID.
        if child.id().is_some() {
            let cleanup = async {
                #[cfg(unix)]
                signal_external_group(id, libc::SIGKILL)?;
                #[cfg(not(unix))]
                child
                    .start_kill()
                    .context("failed to kill external pinger")?;
                child
                    .wait()
                    .await
                    .context("failed to reap external pinger")?;
                anyhow::Ok(())
            }
            .await;
            cleanup.with_context(|| format!("external pinger cleanup failed after: {error:#}"))?;
        }
        return Err(error);
    }
    Ok(())
}

async fn wait_for_external_child(
    child: &mut Child,
    id: u32,
    interrupted: &AtomicBool,
    interrupt: impl Future<Output = anyhow::Result<()>>,
    mut drained: impl Future<Output = anyhow::Result<()>> + Unpin,
) -> anyhow::Result<(ExitStatus, bool)> {
    let mut drain_done = false;
    tokio::select! {
        biased;
        interrupt = interrupt => {
            interrupt?;
            interrupted.store(true, Ordering::Release);
            let status = wait_after_ctrl_c(child, id, &mut drained, drain_done).await?;
            Ok((status, true))
        }
        status = async {
            (&mut drained).await?;
            drain_done = true;
            anyhow::Ok(child.wait().await?)
        } => Ok((status?, false)),
    }
}

async fn wait_after_ctrl_c(
    child: &mut Child,
    _id: u32,
    mut drained: impl Future<Output = anyhow::Result<()>> + Unpin,
    mut drain_done: bool,
) -> anyhow::Result<ExitStatus> {
    #[cfg(unix)]
    signal_external_group(_id, libc::SIGINT)?;
    #[cfg(not(unix))]
    child
        .start_kill()
        .context("failed to stop external pinger after Ctrl-C")?;

    // Defer reaping until pipes close: a leader can exit before its cooperative
    // helpers flush, and its unreaped PID reserves the owned group ID meanwhile.
    let completed = tokio::time::timeout(SHUTDOWN_GRACE, async {
        if !drain_done {
            (&mut drained).await?;
            drain_done = true;
        }
        anyhow::Ok(child.wait().await?)
    })
    .await;
    if let Ok(result) = completed {
        return result;
    }

    #[cfg(unix)]
    signal_external_group(_id, libc::SIGKILL)?;
    #[cfg(not(unix))]
    child
        .start_kill()
        .context("failed to kill external pinger")?;
    let status = child.wait().await?;
    if !drain_done {
        tokio::time::timeout(SHUTDOWN_GRACE, &mut drained)
            .await
            .context("external pinger output did not close after shutdown")??;
    }
    Ok(status)
}

#[cfg(unix)]
fn configure_external_command(command: &mut Command) {
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_external_command(_command: &mut Command) {}

#[cfg(unix)]
fn signal_external_group(id: u32, signal: libc::c_int) -> anyhow::Result<()> {
    let process_group = -(id as libc::pid_t);
    // SAFETY: `kill` only receives the child process group derived from the
    // Tokio child PID and does not dereference any pointers.
    let result = unsafe { libc::kill(process_group, signal) };
    if result == 0 {
        return Ok(());
    }

    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error)
        .with_context(|| format!("failed to send signal {signal} to external pinger group {id}"))
}
