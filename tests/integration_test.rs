use std::{
    io::{BufRead, BufReader, Read},
    net::{TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    process::{Child, Command, ExitStatus, Output as ProcessOutput, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

// This Docker integration test exercises clockping's protocol-facing paths
// against real containers on one Compose network:
// - TCP connect and HTTP HEAD use Python's http.server as the endpoint;
// - native ICMP and external ping verify both ICMP engines inside the network;
// - GTPv1-U, GTPv1-C, and GTPv2-C use the small Python echo responder.
// It intentionally remains one Rust test so Compose setup is paid once while
// each protocol still has an explicit output assertion below.
const DEFAULT_BIN: &str = env!("CARGO_BIN_EXE_clockping");
const RETRIES: usize = 50;
const RETRY_DELAY: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_millis(200);

#[test]
fn exits_nonzero_when_every_probe_fails() {
    let target = unused_local_tcp_addr();
    let output = run_clockping_raw(&[
        "--timestamp",
        "none",
        "tcp",
        "-c",
        "1",
        "-W",
        "0.1",
        &target,
    ]);
    let combined = combined_output(&output);

    assert!(
        !output.status.success(),
        "expected all-loss run to fail\n{combined}"
    );
    assert_contains(&combined, "1 probes transmitted, 0 replies received");
    assert_contains(&combined, "100.0% loss");
}

#[test]
fn broken_stdout_pipe_exits_successfully() {
    let target = unused_local_tcp_addr();
    let bin = clockping_bin();
    let mut child = Command::new(&bin)
        .args([
            "--timestamp",
            "none",
            "tcp",
            "-i",
            "0",
            "-W",
            "0.01",
            &target,
        ])
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
            "--timestamp",
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

#[test]
#[ignore = "requires docker compose test network"]
fn docker_compose_e2e() {
    wait_for_tcp("tcp-target", 8080);
    wait_for_gtp(GtpMode::V1, "gtp-v1u-target", 2152);
    wait_for_gtp(GtpMode::V1, "gtp-v1c-target", 2123);
    wait_for_gtp(GtpMode::V2, "gtp-v2c-target", 2123);

    let tcp_output = run_clockping(&[
        "--timestamp",
        "none",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        "tcp-target:8080",
    ]);
    assert_contains(&tcp_output, "tcp tcp-target:8080 seq=0 reply");
    assert_contains(&tcp_output, "1 probes transmitted, 1 replies received");

    let http_output = run_clockping(&[
        "--timestamp",
        "none",
        "http",
        "-c",
        "1",
        "-W",
        "1",
        "http://tcp-target:8080/",
    ]);
    assert_contains(&http_output, "http http://tcp-target:8080/ seq=0 reply");
    assert_contains(&http_output, "method=HEAD status=200");
    assert_contains(&http_output, "1 probes transmitted, 1 replies received");

    let json_output = run_clockping(&[
        "--timestamp-format",
        "STAMP",
        "--json",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        "tcp-target:8080",
    ]);
    let event = single_json_line(&json_output);
    assert_eq!(event["ts"], "STAMP");
    assert_eq!(event["protocol"], "tcp");
    assert_eq!(event["status"], "reply");
    assert_eq!(event["seq"], 0);
    assert!(event["rtt_ms"].as_f64().is_some_and(|value| value >= 0.0));

    let json_summary_output = run_clockping(&[
        "--timestamp-format",
        "STAMP",
        "--json",
        "tcp",
        "-q",
        "-c",
        "1",
        "-W",
        "1",
        "tcp-target:8080",
    ]);
    let summary = single_json_line(&json_summary_output);
    assert_eq!(summary["type"], "summary");
    assert_eq!(summary["target"], "tcp-target:8080");
    assert_eq!(summary["sent"], 1);
    assert_eq!(summary["received"], 1);
    assert_eq!(summary["lost"], 0);
    assert!(
        summary["rtt_min_ms"]
            .as_f64()
            .is_some_and(|value| value >= 0.0)
    );

    let icmp_output = run_clockping(&[
        "--timestamp",
        "none",
        "icmp",
        "-4",
        "-c",
        "1",
        "-W",
        "1",
        "tcp-target",
    ]);
    assert_contains(&icmp_output, "icmp tcp-target (");
    assert_contains(&icmp_output, "seq=0 reply");
    assert_contains(&icmp_output, "1 probes transmitted, 1 replies received");

    let ping = find_ping();
    let pinger_arg = format!("--pinger={ping}");
    let wrapper_output = run_clockping(&[
        "--timestamp",
        "none",
        "icmp",
        &pinger_arg,
        "-c",
        "1",
        "-W",
        "1",
        "tcp-target",
    ]);
    assert_contains(&wrapper_output, "PING tcp-target");
    assert_contains(&wrapper_output, "1 received");

    let gtp_v1u_output = run_clockping(&[
        "--timestamp",
        "none",
        "gtp",
        "v1u",
        "-c",
        "1",
        "-W",
        "1",
        "gtp-v1u-target",
    ]);
    assert_contains(&gtp_v1u_output, "gtpv1u gtp-v1u-target:2152 seq=0 reply");
    assert_contains(&gtp_v1u_output, "gtp_seq=0");

    let gtp_v1c_output = run_clockping(&[
        "--timestamp",
        "none",
        "gtp",
        "v1c",
        "-c",
        "1",
        "-W",
        "1",
        "gtp-v1c-target",
    ]);
    assert_contains(&gtp_v1c_output, "gtpv1c gtp-v1c-target:2123 seq=0 reply");
    assert_contains(&gtp_v1c_output, "gtp_seq=0");

    let gtp_v2c_output = run_clockping(&[
        "--timestamp",
        "none",
        "gtp",
        "v2c",
        "-c",
        "1",
        "-W",
        "1",
        "gtp-v2c-target",
    ]);
    assert_contains(&gtp_v2c_output, "gtpv2c gtp-v2c-target:2123 seq=0 reply");
    assert_contains(&gtp_v2c_output, "gtp_seq=0");
}

#[derive(Clone, Copy)]
enum GtpMode {
    V1,
    V2,
}

impl GtpMode {
    fn probe_packet(self) -> &'static [u8] {
        match self {
            Self::V1 => &[0x32, 0x01, 0x00, 0x04, 0, 0, 0, 0, 0x12, 0x34, 0, 0],
            Self::V2 => &[0x40, 0x01, 0x00, 0x04, 0x12, 0x34, 0x56, 0],
        }
    }
}

fn wait_for_tcp(host: &str, port: u16) {
    let target = format!("{host}:{port}");
    retry(
        &format!("tcp target did not become ready: {target}"),
        || {
            let mut addresses = target.to_socket_addrs().ok()?;
            addresses
                .any(|address| TcpStream::connect_timeout(&address, PROBE_TIMEOUT).is_ok())
                .then_some(())
        },
    );
}

fn wait_for_gtp(mode: GtpMode, host: &str, port: u16) {
    let target = format!("{host}:{port}");
    retry(
        &format!("gtp target did not become ready: {target}"),
        || {
            let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
            socket.set_read_timeout(Some(PROBE_TIMEOUT)).ok()?;
            socket.send_to(mode.probe_packet(), &target).ok()?;

            let mut buf = [0_u8; 2048];
            let (len, _peer) = socket.recv_from(&mut buf).ok()?;
            (len >= 2 && buf[1] == 2).then_some(())
        },
    );
}

fn retry(message: &str, mut probe: impl FnMut() -> Option<()>) {
    for _ in 0..RETRIES {
        if probe().is_some() {
            return;
        }
        thread::sleep(RETRY_DELAY);
    }
    panic!("{message}");
}

fn run_clockping(args: &[&str]) -> String {
    let bin = clockping_bin();
    eprintln!("+ {bin} {}", args.join(" "));
    let output = run_clockping_raw(args);
    let combined = combined_output(&output);
    eprintln!("{combined}");

    assert!(
        output.status.success(),
        "clockping failed with status {}\n{}",
        output.status,
        combined
    );
    combined
}

fn run_clockping_raw(args: &[&str]) -> ProcessOutput {
    Command::new(clockping_bin())
        .args(args)
        .output()
        .expect("failed to spawn clockping")
}

fn clockping_bin() -> String {
    std::env::var("CLOCKPING_BIN").unwrap_or_else(|_| DEFAULT_BIN.to_string())
}

fn combined_output(output: &ProcessOutput) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

fn unused_local_tcp_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind local TCP listener");
    let addr = listener.local_addr().expect("failed to read local address");
    drop(listener);
    addr.to_string()
}

fn wait_for_child(child: &mut Child, timeout: Duration) -> ExitStatus {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("failed to poll child") {
            return status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            panic!("child did not exit within {timeout:?}");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn child_stdout(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_string(&mut output)
            .expect("failed to read child stdout");
    }
    output
}

fn child_stderr(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        stderr
            .read_to_string(&mut output)
            .expect("failed to read child stderr");
    }
    output
}

fn single_json_line(output: &str) -> Value {
    let lines = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1, "expected exactly one JSON line: {output}");
    serde_json::from_str(lines[0]).expect("invalid JSON output")
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected output to contain {needle:?}\n{haystack}"
    );
}

fn find_ping() -> &'static str {
    ["/usr/bin/ping", "/bin/ping"]
        .into_iter()
        .find(|path| std::path::Path::new(path).exists())
        .expect("ping binary not found")
}
