use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

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
    let started = Instant::now();
    let mut child = Command::new(&bin)
        .args([
            "--ts.preset",
            "none",
            "gtp",
            "v1u",
            "-W",
            "10",
            "-i",
            "10",
            "127.0.0.1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clockping");

    // Allow the binary to enter the probe loop so this verifies interruption
    // of an active probe rather than process startup signal handling.
    thread::sleep(Duration::from_secs(1));
    let signal_status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("failed to send SIGINT");
    assert!(signal_status.success(), "failed to send SIGINT");

    let status = wait_for_child(&mut child, Duration::from_secs(3));
    let stdout = child_stdout(&mut child);
    let stderr = child_stderr(&mut child);
    assert!(
        status.success(),
        "expected SIGINT summary exit to succeed, got {status}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(4),
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

#[cfg(unix)]
#[test]
fn external_pinger_sigint_drains_stats_without_timestamps() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger SIGINT test; python3 not found");
        return;
    };
    let script = r#"
import signal
import sys
import time

def stop(signum, frame):
    print()
    print('--- mock ping statistics ---')
    print('3 packets transmitted, 3 received, 0% packet loss')
    sys.stdout.flush()
    sys.exit(0)

signal.signal(signal.SIGINT, stop)
print('PING mock (127.0.0.1): 56 data bytes', flush=True)
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
    set_own_process_group(&mut command);
    let mut child = command.spawn().expect("failed to spawn clockping");

    let stdout = child.stdout.take().expect("missing stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut stdout = String::new();
    let bytes = reader
        .read_line(&mut stdout)
        .expect("failed to read first external pinger line");
    assert_ne!(bytes, 0, "clockping exited before mock pinger output");
    assert_contains(&stdout, "STAMP PING mock");

    interrupt_process_group(child.id());

    let status = wait_for_child(&mut child, Duration::from_secs(5));
    reader
        .read_to_string(&mut stdout)
        .expect("failed to read remaining external pinger output");
    let stderr = child_stderr(&mut child);

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

#[test]
fn completion_subcommand_generates_bash_script() {
    let output = run_clockping_raw(&["completion", "bash"]);
    let combined = combined_output(&output);

    assert!(
        output.status.success(),
        "completion generation failed with status {}\n{}",
        output.status,
        combined
    );
    assert_contains(&combined, "_clockping");
    assert_contains(&combined, "tcp");
    assert_contains(&combined, "http");
    assert_contains(&combined, "gtp");
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
