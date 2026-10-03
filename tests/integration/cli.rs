use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::net::UdpSocket;

use super::helpers::*;

#[test]
fn exits_nonzero_when_every_probe_fails() {
    let target = unreachable_tcp_target();
    let output = run_clockping_raw(&["--ts.preset", "none", "tcp", "-c", "1", "-W", "0.1", target]);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "expected all-loss run to fail\n{combined}"
    );
    assert_contains(&combined, "1 probes transmitted, 0 replies received");
    assert_contains(&combined, "100.0% loss");
}

#[test]
fn tcp_target_requires_explicit_port() {
    let output = run_clockping_raw(&["tcp", "example.com"]);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "missing TCP port should fail\n{combined}"
    );
    assert_contains(&combined, "TCP target must include a port");
}

#[test]
fn invalid_timestamp_format_fails_before_metrics_file_is_opened() {
    let metrics_file = temp_metrics_path("jsonl");
    fs::write(&metrics_file, "sentinel").unwrap();
    let path = metrics_file.to_str().unwrap();
    for format in ["%Q", "%", "%#z"] {
        let output = run_clockping_raw(&[
            "--metrics.file",
            path,
            "--ts.format",
            format,
            "tcp",
            "-c",
            "0",
            "127.0.0.1:1",
        ]);
        assert!(!output.status.success());
        assert_contains(&combined_output(&output), "invalid --ts.format");
        assert_eq!(fs::read_to_string(&metrics_file).unwrap(), "sentinel");

        let output = run_clockping_raw(&[
            "--ts.format",
            format,
            "icmp",
            "--pinger",
            "/nonexistent-clockping-pinger",
        ]);
        assert!(!output.status.success());
        assert_contains(&combined_output(&output), "invalid --ts.format");
    }
    fs::remove_file(&metrics_file).unwrap();

    let output = run_clockping_raw(&[
        "--ts.preset",
        "none",
        "--ts.format",
        "STAMP",
        "--out.format",
        "json",
        "tcp",
        "-c",
        "0",
        "127.0.0.1:1",
    ]);
    assert!(output.status.success(), "{}", combined_output(&output));
}

#[test]
fn tcp_ipv4_flag_probes_ipv4_target() {
    let target = spawn_tcp_acceptor(1);

    let output = run_clockping(&[
        "--ts.preset",
        "none",
        "tcp",
        "-4",
        "-c",
        "1",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, &format!("tcp {target} seq=0 reply"));
}

#[test]
fn http_ipv4_flag_probes_ipv4_target() {
    let target = spawn_http_responder(1);
    let url = format!("http://{target}/");

    let output = run_clockping(&[
        "--ts.preset",
        "none",
        "http",
        "-4",
        "-c",
        "1",
        "-W",
        "1",
        &url,
    ]);

    assert_contains(&output, &format!("http {url} seq=0 reply"));
    assert_contains(&output, "method=HEAD status=200");
}

#[test]
fn summaries_are_printed_after_all_parallel_target_events() {
    let fast = spawn_http_responder(1);
    let slow = spawn_delayed_http_responder(1, Duration::from_millis(150));
    let fast_url = format!("http://{fast}/");
    let slow_url = format!("http://{slow}/");

    let output = run_clockping(&[
        "--ts.preset",
        "none",
        "http",
        "-c",
        "1",
        "-W",
        "2",
        &fast_url,
        &slow_url,
    ]);

    let first_stats = output
        .find("clockping statistics")
        .expect("missing summary output");
    let fast_reply = output
        .find(&format!("http {fast_url} seq=0 reply"))
        .expect("missing fast target reply");
    let slow_reply = output
        .find(&format!("http {slow_url} seq=0 reply"))
        .expect("missing slow target reply");

    assert!(
        fast_reply < first_stats && slow_reply < first_stats,
        "all target events should be printed before the first summary\n{output}"
    );
    assert!(
        !output[first_stats..].contains(" seq="),
        "summary output should not be followed by more probe event lines\n{output}"
    );
    let fast_summary = output
        .find(&format!("--- {fast_url} clockping statistics ---"))
        .expect("missing fast target summary");
    let slow_summary = output
        .find(&format!("--- {slow_url} clockping statistics ---"))
        .expect("missing slow target summary");
    assert!(
        fast_summary < slow_summary,
        "summaries should keep input target order\n{output}"
    );
}

#[test]
fn broken_stdout_pipe_exits_successfully() {
    let target = spawn_tcp_acceptor(1000);
    let bin = clockping_bin();
    let mut child = Command::new(&bin)
        .args(["--ts.preset", "none", "tcp", "-i", "0", "-W", "1", &target])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clockping");

    let stdout = child.stdout.take().expect("missing stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .expect("failed to read first output line");
    assert_contains(&first_line, "tcp ");
    drop(reader);

    let status = wait_for_child(&mut child, Duration::from_secs(3));
    let stderr = child_stderr(&mut child);
    assert!(
        status.success(),
        "expected broken pipe to exit successfully, got {status}\n{stderr}"
    );
    assert!(
        !stderr.contains("panicked"),
        "broken pipe should not panic\n{stderr}"
    );
}

#[cfg(unix)]
#[test]
fn sigint_interrupts_active_probe() {
    let bin = clockping_bin();
    let server = UdpSocket::bind("127.0.0.1:0").expect("failed to bind silent GTP target");
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let port = server.local_addr().unwrap().port().to_string();
    let mut command = Command::new(&bin);
    command
        .args([
            "--ts.preset",
            "none",
            "gtp",
            "v1u",
            "-W",
            "10",
            "-i",
            "10",
            "--port",
            &port,
            "127.0.0.1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = GuardedChild::spawn(command);

    // The runner polls its signal listener before sending this real request.
    // This handshake proves startup finished and a silent long-timeout probe is active.
    let mut request = [0; 64];
    server
        .recv_from(&mut request)
        .expect("clockping did not send its first probe");
    let started = Instant::now();
    interrupt_process_group(child.child.id());

    let status = wait_for_child(&mut child.child, Duration::from_secs(3));
    let stdout = child_stdout(&mut child.child);
    let stderr = child_stderr(&mut child.child);
    assert!(
        status.success(),
        "expected SIGINT summary exit to succeed, got {status}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "SIGINT did not interrupt the active probe promptly"
    );
    assert_contains(&stdout, "clockping statistics");
    assert!(
        !stderr.contains("panicked"),
        "SIGINT should not panic\n{stderr}"
    );
}

#[test]
fn external_pinger_failure_prints_stderr_without_timestamp() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger stderr test; python3 not found");
        return;
    };
    let script =
        "import sys\nprint('usage: mock ping', file=sys.stderr, flush=True)\nsys.exit(64)\n";

    let output = run_clockping_raw(&[
        "--ts.format",
        "STAMP",
        "icmp",
        "--pinger",
        python,
        "-c",
        script,
    ]);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "failing mock pinger should fail clockping\n{combined}"
    );
    assert_contains(&combined, "usage: mock ping");
    assert!(
        !combined.contains("STAMP usage: mock ping"),
        "external pinger stderr should not be timestamped\n{combined}"
    );
}

#[test]
fn external_pinger_natural_exit_preserves_arguments_and_output() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger natural exit test; python3 not found");
        return;
    };
    let script = "import sys; print(sys.argv[1], flush=True); print('diagnostic', file=sys.stderr, flush=True)";
    let output = run_clockping_raw(&[
        "--ts.format",
        "STAMP",
        "icmp",
        "--pinger",
        python,
        "-c",
        script,
        "argument with spaces",
    ]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "{}\n{stdout}\n{stderr}",
        output.status
    );
    assert_contains(&stdout, "STAMP argument with spaces");
    assert_eq!(stderr.trim(), "diagnostic");
}

#[cfg(unix)]
#[test]
fn external_pinger_sigint_drains_stats_without_timestamps() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger SIGINT test; python3 not found");
        return;
    };
    let script = r#"
import os
import signal
import sys
import time

def stop(signum, frame):
    time.sleep(0.1) # Cooperative children get time to flush their statistics.
    print()
    print('--- mock ping statistics ---')
    print('3 packets transmitted, 3 received, 0% packet loss')
    sys.stdout.flush()
    sys.exit(0)

signal.signal(signal.SIGINT, stop)
# Avoid interrupting Python's buffered stdout lock from the signal handler.
os.write(1, b'PING mock (127.0.0.1): 56 data bytes\n')
while True:
    time.sleep(10)
"#;
    let bin = clockping_bin();
    let mut command = Command::new(&bin);
    command
        .args([
            "--ts.format",
            "STAMP",
            "icmp",
            "--pinger",
            python,
            "-c",
            script,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = GuardedChild::spawn(command);
    let lines = child_stdout_lines(&mut child.child);
    let mut stdout = next_stdout_line(&lines);
    assert_contains(&stdout, "STAMP PING mock");

    interrupt_process_group(child.child.id());

    let status = wait_for_child(&mut child.child, Duration::from_secs(5));
    stdout.push('\n');
    stdout.push_str(&remaining_stdout(&lines));
    let stderr = child_stderr(&mut child.child);

    assert!(
        status.success(),
        "expected external pinger SIGINT to exit successfully, got {status}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_contains(&stdout, "mock ping statistics");
    assert_contains(&stdout, "3 packets transmitted, 3 received");
    assert!(
        !stdout.contains("STAMP --- mock ping statistics"),
        "external pinger stats should not be timestamped after Ctrl-C\n{stdout}"
    );
    assert!(
        !stdout.contains("STAMP 3 packets transmitted"),
        "external pinger stats should not be timestamped after Ctrl-C\n{stdout}"
    );
}

#[cfg(unix)]
#[test]
fn external_pinger_sigint_bounds_shutdown_and_cleans_owned_descendants() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger shutdown test; python3 not found");
        return;
    };
    let script = r#"
import os, signal, socket, sys, time
signal.signal(signal.SIGINT, signal.SIG_IGN)
parent_ready, child_ready = socket.socketpair()
descendant = os.fork()
if descendant == 0:
    parent_ready.close()
    def delayed_stats(signum, frame):
        assert child_ready.recv(1) == b'' # Leader exited and closed its socket.
        time.sleep(0.1)
        os.write(1, b'helper final statistics\n')
        os.write(2, b'helper stderr statistics\n')
        os._exit(0)
    if sys.argv[1] == 'delayed':
        signal.signal(signal.SIGINT, delayed_stats)
    child_ready.send(b'!') # Handler installed before the public readiness line.
    if sys.argv[1] == 'exit':
        assert child_ready.recv(1) == b''
        os.write(1, b'LEADER_EXITED\n')
    while True:
        time.sleep(10)
child_ready.close()
assert parent_ready.recv(1) == b'!'
def stop(signum, frame):
    print('final statistics', flush=True)
    sys.exit(0)
if sys.argv[1] in ('cooperate', 'delayed'):
    signal.signal(signal.SIGINT, stop)
os.write(1, f'READY {os.getpid()} {descendant}\n'.encode())
if sys.argv[1] == 'exit':
    sys.exit(0)
while True:
    time.sleep(10)
"#;
    let mut sentinel_command = Command::new(python);
    sentinel_command
        .args([
            "-c",
            "import time; print('UNOWNED', flush=True); time.sleep(30)",
        ])
        .stdout(Stdio::piped());
    let mut sentinel = GuardedChild::spawn(sentinel_command);
    let sentinel_lines = child_stdout_lines(&mut sentinel.child);
    assert_eq!(next_stdout_line(&sentinel_lines), "UNOWNED");

    for mode in ["delayed", "ignore", "cooperate", "exit"] {
        let mut command = Command::new(clockping_bin());
        command
            .args([
                "--ts.format",
                "STAMP",
                "icmp",
                "--pinger",
                python,
                "-c",
                script,
                mode,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = GuardedChild::spawn(command);
        let lines = child_stdout_lines(&mut child.child);
        let first = next_stdout_line(&lines);
        let fields = first
            .strip_prefix("STAMP ")
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>();
        let pinger: u32 = fields[1].parse().unwrap();
        child.external_groups.push(pinger);
        let descendant: u32 = fields[2].parse().unwrap();
        assert_eq!(fields[0], "READY");
        if mode == "exit" {
            assert_eq!(next_stdout_line(&lines), "STAMP LEADER_EXITED");
        }
        let started = Instant::now();
        interrupt_process_group(child.child.id());
        let status = wait_for_child(&mut child.child, Duration::from_secs(3));
        let stdout = remaining_stdout(&lines);
        let stderr = child_stderr(&mut child.child);
        assert!(status.success(), "{mode}: {status}\n{stdout}\n{stderr}");
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "{mode}: shutdown overran its grace"
        );
        if mode == "cooperate" || mode == "delayed" {
            assert!(
                stdout.contains("final statistics"),
                "cooperative statistics missing after {:?}; status={status}, stdout={stdout:?}, stderr={stderr:?}",
                started.elapsed()
            );
        }
        if mode == "delayed" {
            assert_contains(&stdout, "helper final statistics");
            assert_contains(&stderr, "helper stderr statistics");
            assert!(!stdout.contains("STAMP helper final statistics"));
            assert!(!stderr.contains("STAMP helper stderr statistics"));
        }
        assert_process_gone(pinger); // Direct child is reaped, not merely signaled.
        assert_process_gone(descendant);
        assert!(
            sentinel.child.try_wait().unwrap().is_none(),
            "an unowned process was signaled"
        );
        child.external_groups.clear();
    }
    sentinel.child.kill().unwrap();
    sentinel.child.wait().unwrap();
}

#[cfg(unix)]
#[test]
fn external_pinger_broken_output_cleans_owned_descendants() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger BrokenPipe test; python3 not found");
        return;
    };
    let script = r#"
import os, signal, sys, time
signal.signal(signal.SIGINT, signal.SIG_IGN)
descendant = os.fork()
if descendant == 0:
    while True:
        time.sleep(10)
os.write(1, f'READY {os.getpid()} {descendant}\n'.encode())
while True:
    os.write(1 if sys.argv[1] == 'stdout' else 2, b'probe output\n')
    time.sleep(0.001)
"#;
    let mut sentinel_command = Command::new(python);
    sentinel_command
        .args([
            "-c",
            "import time; print('UNOWNED', flush=True); time.sleep(30)",
        ])
        .stdout(Stdio::piped());
    let mut sentinel = GuardedChild::spawn(sentinel_command);
    let sentinel_lines = child_stdout_lines(&mut sentinel.child);
    assert_eq!(next_stdout_line(&sentinel_lines), "UNOWNED");

    for stream in ["stdout", "stderr"] {
        let mut command = Command::new(clockping_bin());
        command
            .args([
                "--ts.preset",
                "none",
                "icmp",
                "--pinger",
                python,
                "-c",
                script,
                stream,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = GuardedChild::spawn(command);
        let stdout = child.child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let read = reader.read_line(&mut line);
            let _ = tx.send((read, line, reader));
        });
        let (read, first, reader) = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("external pinger did not produce its readiness line");
        assert_ne!(read.unwrap(), 0);
        let fields = first.split_whitespace().collect::<Vec<_>>();
        let pinger: u32 = fields[1].parse().unwrap();
        child.external_groups.push(pinger);
        let descendant: u32 = fields[2].parse().unwrap();
        assert_eq!(fields[0], "READY");
        let reader = if stream == "stdout" {
            drop(reader); // Close wrapper stdout only after real readiness.
            None
        } else {
            drop(child.child.stderr.take().unwrap());
            Some(reader) // stdout remains open and its child pipe never reaches EOF.
        };

        let status = wait_for_child(&mut child.child, Duration::from_secs(3));
        let stderr = child_stderr(&mut child.child);
        assert!(
            status.success(),
            "{stream}: BrokenPipe must remain a successful exit: {status}\n{stderr}"
        );
        assert_process_gone(pinger);
        assert_process_gone(descendant);
        assert!(
            sentinel.child.try_wait().unwrap().is_none(),
            "an unowned process was signaled"
        );
        child.external_groups.clear();
        drop(reader);
    }
    sentinel.child.kill().unwrap();
    sentinel.child.wait().unwrap();
}

#[test]
fn completion_subcommand_matches_tracked_scripts() {
    for (shell, script) in [
        (
            "bash",
            include_bytes!("../../completions/clockping.bash").as_slice(),
        ),
        (
            "zsh",
            include_bytes!("../../completions/_clockping").as_slice(),
        ),
        (
            "fish",
            include_bytes!("../../completions/clockping.fish").as_slice(),
        ),
    ] {
        let output = run_clockping_raw(&["completion", shell]);
        assert!(
            output.status.success(),
            "{shell} completion generation failed: {}",
            combined_output(&output)
        );
        assert_eq!(output.stdout, script, "{shell} completion is stale");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn icmp_help_lists_native_options() {
    for args in [["icmp", "--help"], ["help", "icmp"]] {
        let output = run_clockping_raw(&args);
        let combined = combined_output(&output);

        assert!(
            output.status.success(),
            "icmp help failed with status {}\n{}",
            output.status,
            combined
        );
        assert_contains(&combined, "Usage: clockping icmp [OPTIONS] <DESTINATION>");
        assert_contains(&combined, "-c, --count <COUNT>");
        assert_contains(&combined, "-i, --interval <SECONDS>");
        assert_contains(&combined, "-W, --timeout <SECONDS>");
        assert_contains(&combined, "-I, --interface-or-source <INTERFACE_OR_SOURCE>");
        assert_contains(&combined, "-D, --timestamp");
        assert_contains(&combined, "--pinger <PROGRAM>");
        assert_contains(&combined, "Metrics Options:");
        assert_contains(&combined, "--metrics.file <PATH>");
        assert!(
            !combined.contains("raw argv layer"),
            "help should describe user-facing options, not parser internals\n{combined}"
        );
    }
}

#[test]
fn mode_help_lists_global_options() {
    let cases: &[&[&str]] = &[
        &["icmp", "--help"],
        &["tcp", "--help"],
        &["http", "--help"],
        &["gtp", "--help"],
        &["gtp", "v1u", "--help"],
    ];

    for args in cases {
        let output = run_clockping_raw(args);
        let combined = combined_output(&output);

        assert!(
            output.status.success(),
            "mode help failed for {:?} with status {}\n{}",
            args,
            output.status,
            combined
        );
        assert_contains(&combined, "-V, --version");
        for expected in [
            "Output Options:",
            "--ts.preset <PRESET>",
            "--ts.format <FORMAT>",
            "--out.format <FORMAT>",
            "--out.colored",
            "Metrics Options:",
            "--push.url <URL>",
            "--push.delete-on-exit",
            "--push.interval <DURATION>",
            "--push.job <JOB>",
            "--push.label <KEY=VALUE>",
            "--push.retries <N>",
            "--push.timeout <DURATION>",
            "--push.user-agent <VALUE>",
            "--metrics.file <PATH>",
            "--metrics.format <FORMAT>",
            "--metrics.label <KEY=VALUE>",
            "--metrics.prefix <PREFIX>",
            "CLI labels override same-name environment defaults",
        ] {
            assert_contains(&combined, expected);
        }
        for removed in [
            "--timestamp <TIMESTAMP>",
            "--timestamp-format",
            "--timestamp.preset",
            "--timestamp.format",
            "--json",
            "--colored",
            "--output.format",
            "--output.color",
            "--out.color <WHEN>",
        ] {
            assert!(
                !combined.contains(removed),
                "mode help should not show removed output option {removed}\n{combined}"
            );
        }
    }
}

#[test]
fn tcp_probes_multiple_targets() {
    let first = spawn_tcp_acceptor(1);
    let second = spawn_tcp_acceptor(1);

    let output = run_clockping(&[
        "--ts.preset",
        "none",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &first,
        &second,
    ]);

    assert_contains(&output, &format!("tcp {first} seq=0 reply"));
    assert_contains(&output, &format!("tcp {second} seq=0 reply"));
    assert_contains(
        &output,
        &format!("--- {first} clockping statistics ---\n1 probes transmitted"),
    );
    assert_contains(
        &output,
        &format!("--- {second} clockping statistics ---\n1 probes transmitted"),
    );
}

#[test]
fn colored_output_uses_ansi_escape_sequences() {
    let target = spawn_tcp_acceptor(1);

    let output = run_clockping(&[
        "--out.colored",
        "--ts.preset",
        "none",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, "\x1b[34m");
    assert_contains(&output, "\x1b[32mreply\x1b[0m");
    assert_contains(&output, "\x1b[32m0.0% loss\x1b[0m");
}

#[test]
fn version_includes_build_metadata() {
    let output = run_clockping_raw(&["--version"]);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "version failed with status {}\n{}",
        output.status,
        combined
    );
    assert_contains(&combined, "clockping ");
    assert_contains(&combined, "(git ");
    assert_contains(&combined, "commit ");
    assert_contains(&combined, "commit date ");
    assert_contains(&combined, "built ");
    assert_contains(&combined, " on ");
    assert_contains(&combined, "(host ");
}
