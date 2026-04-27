use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    process::{Child, Command, ExitStatus, Output as ProcessOutput, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

// This Docker integration test exercises clockping's protocol-facing paths
// against real containers on one Compose network:
// - TCP connect and HTTP HEAD use Python's http.server as the endpoint;
// - transient TCP, HTTP, ICMP, and GTP targets verify that target-down probes
//   keep producing timestamped events instead of leaving a silent terminal;
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
        "--timestamp",
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
        "--timestamp",
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
fn external_pinger_failure_prints_stderr_without_timestamp() {
    let Some(python) = find_python3() else {
        eprintln!("skipping external pinger stderr test; python3 not found");
        return;
    };
    let script =
        "import sys\nprint('usage: mock ping', file=sys.stderr, flush=True)\nsys.exit(64)\n";

    let output = run_clockping_raw(&[
        "--timestamp-format",
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
    let mut child = Command::new(&bin)
        .args([
            "--timestamp-format",
            "STAMP",
            "icmp",
            "--pinger",
            python,
            "-c",
            script,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clockping");

    let stdout = child.stdout.take().expect("missing stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut stdout = String::new();
    let bytes = reader
        .read_line(&mut stdout)
        .expect("failed to read first external pinger line");
    assert_ne!(bytes, 0, "clockping exited before mock pinger output");
    assert_contains(&stdout, "STAMP PING mock");

    let signal_status = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("failed to send SIGINT");
    assert!(signal_status.success(), "failed to send SIGINT");

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
        assert_contains(&combined, "--pinger <PROGRAM>");
        assert!(
            !combined.contains("raw argv layer"),
            "help should describe user-facing options, not parser internals\n{combined}"
        );
    }
}

#[test]
fn writes_jsonl_metrics_file_without_replacing_stdout() {
    let target = spawn_tcp_acceptor(2);
    let metrics_file = temp_metrics_path("jsonl");
    let metrics_file_arg = metrics_file.to_string_lossy();

    let output = run_clockping(&[
        "tcp",
        "--metrics.file",
        metrics_file_arg.as_ref(),
        "-c",
        "2",
        "-i",
        "0",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, "2 probes transmitted, 2 replies received");
    let metrics = fs::read_to_string(&metrics_file).expect("read metrics file");
    let lines = metrics.lines().collect::<Vec<_>>();
    assert_eq!(
        lines.len(),
        2,
        "expected one metrics line per probe: {metrics}"
    );
    let first: Value = serde_json::from_str(lines[0]).expect("parse first metrics line");
    let second: Value = serde_json::from_str(lines[1]).expect("parse second metrics line");
    assert_eq!(first["schema_version"], 1);
    assert_eq!(first["event"], "interval");
    assert_eq!(first["protocol"], "tcp");
    assert_eq!(first["sent"], 1);
    assert_eq!(second["sent"], 2);
    assert_eq!(second["received"], 2);
    let _ = fs::remove_file(metrics_file);
}

#[test]
fn writes_prometheus_metrics_file_with_labels() {
    let target = spawn_tcp_acceptor(1);
    let metrics_file = temp_metrics_path("prom");
    let metrics_file_arg = metrics_file.to_string_lossy();

    let output = run_clockping(&[
        "--metrics.file",
        metrics_file_arg.as_ref(),
        "--metrics.format",
        "prometheus",
        "--metrics.prefix",
        "nettest",
        "--metrics.label",
        "site=ci",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, "1 probes transmitted, 1 replies received");
    let metrics = fs::read_to_string(&metrics_file).expect("read metrics file");
    assert_contains(
        &metrics,
        "nettest_probe_sent{site=\"ci\",protocol=\"tcp\",target=\"",
    );
    assert_contains(&metrics, "nettest_probe_rtt_seconds");
    assert!(!metrics.contains(r#""event":"interval""#));
    let _ = fs::remove_file(metrics_file);
}

#[test]
fn tcp_probes_multiple_targets() {
    let first = spawn_tcp_acceptor(1);
    let second = spawn_tcp_acceptor(1);

    let output = run_clockping(&[
        "--timestamp",
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
        "--colored",
        "--timestamp",
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
fn prometheus_metrics_file_includes_multiple_targets() {
    let first = spawn_tcp_acceptor(1);
    let second = spawn_tcp_acceptor(1);
    let metrics_file = temp_metrics_path("prom");
    let metrics_file_arg = metrics_file.to_string_lossy();

    let output = run_clockping(&[
        "--metrics.file",
        metrics_file_arg.as_ref(),
        "--metrics.format",
        "prometheus",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &first,
        &second,
    ]);

    assert_contains(&output, &format!("tcp {first}"));
    assert_contains(&output, &format!("tcp {second}"));
    let metrics = fs::read_to_string(&metrics_file).expect("read metrics file");
    assert_contains(&metrics, &format!("target=\"{first}\""));
    assert_contains(&metrics, &format!("target=\"{second}\""));
    let _ = fs::remove_file(metrics_file);
}

#[test]
fn pushes_prometheus_metrics_to_pushgateway() {
    let target = spawn_tcp_acceptor(1);
    let (push_url, requests) = spawn_pushgateway_capture();

    let output = run_clockping(&[
        "--push.url",
        &push_url,
        "--push.job",
        "clock job",
        "--push.label",
        "scenario=push test",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, "1 probes transmitted, 1 replies received");
    let request = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("Pushgateway request was not captured");
    assert_contains(
        &request.request_line,
        "PUT /metrics/job/clock%20job/scenario/push%20test HTTP/1.1",
    );
    assert_contains(
        &request.body,
        "clockping_probe_sent{protocol=\"tcp\",target=\"",
    );
    assert_contains(&request.body, "clockping_probe_up");
}

#[test]
fn pushgateway_metrics_include_multiple_targets() {
    let first = spawn_tcp_acceptor(1);
    let second = spawn_tcp_acceptor(1);
    let (push_url, requests) = spawn_pushgateway_capture_n(2);

    let output = run_clockping(&[
        "--push.url",
        &push_url,
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &first,
        &second,
    ]);

    assert_contains(&output, &format!("tcp {first}"));
    assert_contains(&output, &format!("tcp {second}"));
    let captured = (0..2)
        .map(|_| {
            requests
                .recv_timeout(Duration::from_secs(3))
                .expect("Pushgateway request was not captured")
        })
        .collect::<Vec<_>>();
    let body = captured
        .iter()
        .map(|request| request.body.as_str())
        .find(|body| {
            body.contains(&format!("target=\"{first}\""))
                && body.contains(&format!("target=\"{second}\""))
        })
        .unwrap_or_else(|| panic!("missing combined multi-target Pushgateway body: {captured:?}"));
    assert_contains(body, "clockping_probe_sent");
    assert_contains(body, "clockping_probe_up");
}

#[test]
fn pushes_window_metrics_to_pushgateway() {
    let target = spawn_tcp_acceptor(2);
    let (push_url, requests) = spawn_pushgateway_capture();

    let output = run_clockping(&[
        "--push.url",
        &push_url,
        "--push.interval",
        "10s",
        "tcp",
        "-c",
        "2",
        "-i",
        "0",
        "-W",
        "1",
        &target,
    ]);

    assert_contains(&output, "2 probes transmitted, 2 replies received");
    let request = requests
        .recv_timeout(Duration::from_secs(3))
        .expect("Pushgateway window request was not captured");
    assert_contains(&request.request_line, "PUT /metrics/job/clockping HTTP/1.1");
    assert_contains(&request.body, "clockping_window_samples");
    assert_contains(&request.body, "clockping_window_replies");
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
    wait_for_tcp("transient-tcp-target", 8081);
    wait_for_tcp("transient-http-target", 8082);
    wait_for_tcp("transient-icmp-target", 9090);
    wait_for_gtp(GtpMode::V1, "gtp-v1u-target", 2152);
    wait_for_gtp(GtpMode::V1, "transient-gtp-v1u-target", 2152);
    wait_for_gtp(GtpMode::V1, "transient-gtp-v1c-target", 2123);
    wait_for_gtp(GtpMode::V2, "transient-gtp-v2c-target", 2123);
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

    let down_output = run_clockping(&[
        "--timestamp-format",
        "STAMP",
        "--json",
        "tcp",
        "-c",
        "4",
        "-i",
        "0.2",
        "-W",
        "0.1",
        "transient-tcp-target:8081",
    ]);
    assert_timestamped_target_down_events(&down_output, "tcp");

    let http_down_output = run_clockping(&[
        "--timestamp-format",
        "STAMP",
        "--json",
        "http",
        "-c",
        "4",
        "-i",
        "0.2",
        "-W",
        "0.1",
        "http://transient-http-target:8082/",
    ]);
    assert_timestamped_target_down_events(&http_down_output, "http");

    for (variant, target, protocol) in [
        ("v1u", "transient-gtp-v1u-target", "gtpv1u"),
        ("v1c", "transient-gtp-v1c-target", "gtpv1c"),
        ("v2c", "transient-gtp-v2c-target", "gtpv2c"),
    ] {
        let gtp_down_output = run_clockping(&[
            "--timestamp-format",
            "STAMP",
            "--json",
            "gtp",
            variant,
            "-c",
            "4",
            "-i",
            "0.2",
            "-W",
            "0.1",
            target,
        ]);
        assert_timestamped_target_down_events(&gtp_down_output, protocol);
    }

    let icmp_down_output = run_clockping_after_first_line(
        &[
            "--timestamp-format",
            "STAMP",
            "--json",
            "icmp",
            "-4",
            "-c",
            "4",
            "-i",
            "0.3",
            "-W",
            "0.2",
            "transient-icmp-target",
        ],
        |_| trigger_icmp_down("transient-icmp-target", 9090),
    );
    assert_timestamped_target_down_events(&icmp_down_output, "icmp");

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

fn run_clockping_after_first_line(args: &[&str], after_first_line: impl FnOnce(&str)) -> String {
    let bin = clockping_bin();
    eprintln!("+ {bin} {}", args.join(" "));
    let mut child = Command::new(&bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn clockping");

    let stdout = child.stdout.take().expect("missing stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut output = String::new();
    let bytes = reader
        .read_line(&mut output)
        .expect("failed to read first output line");
    assert_ne!(bytes, 0, "clockping exited before writing an event");

    after_first_line(&output);
    let status = wait_for_child(&mut child, Duration::from_secs(8));
    reader
        .read_to_string(&mut output)
        .expect("failed to read remaining output");
    let stderr = child_stderr(&mut child);
    eprintln!("{output}{stderr}");

    assert!(
        status.success(),
        "clockping failed with status {status}\n{output}{stderr}"
    );
    format!("{output}{stderr}")
}

fn clockping_bin() -> String {
    std::env::var("CLOCKPING_BIN").unwrap_or_else(|_| DEFAULT_BIN.to_string())
}

fn combined_output(output: &ProcessOutput) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

fn trigger_icmp_down(host: &str, port: u16) {
    let mut stream = TcpStream::connect((host, port)).expect("failed to connect to ICMP control");
    stream
        .write_all(b"down")
        .expect("failed to write ICMP down command");
    stream
        .shutdown(Shutdown::Write)
        .expect("failed to close ICMP control write side");
    thread::sleep(Duration::from_millis(100));
}

fn unused_local_tcp_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind local TCP listener");
    let addr = listener.local_addr().expect("failed to read local address");
    drop(listener);
    addr.to_string()
}

fn spawn_tcp_acceptor(accepts: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind TCP acceptor");
    let addr = listener
        .local_addr()
        .expect("failed to read TCP acceptor address");
    thread::spawn(move || {
        for _ in 0..accepts {
            if listener.accept().is_err() {
                break;
            }
        }
    });
    addr.to_string()
}

fn spawn_http_responder(accepts: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind HTTP responder");
    let addr = listener
        .local_addr()
        .expect("failed to read HTTP responder address");
    thread::spawn(move || {
        for _ in 0..accepts {
            let Ok((mut stream, _peer)) = listener.accept() else {
                break;
            };
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    });
    addr.to_string()
}

#[derive(Debug)]
struct CapturedHttpRequest {
    request_line: String,
    body: String,
}

fn spawn_pushgateway_capture() -> (String, mpsc::Receiver<CapturedHttpRequest>) {
    spawn_pushgateway_capture_n(1)
}

fn spawn_pushgateway_capture_n(count: usize) -> (String, mpsc::Receiver<CapturedHttpRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind Pushgateway capture");
    let addr = listener
        .local_addr()
        .expect("failed to read Pushgateway capture address");
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        for _ in 0..count {
            let Ok((stream, _peer)) = listener.accept() else {
                return;
            };
            let Some(request) = capture_pushgateway_request(stream) else {
                return;
            };
            let _ = tx.send(request);
        }
    });

    (format!("http://{addr}"), rx)
}

fn capture_pushgateway_request(mut stream: TcpStream) -> Option<CapturedHttpRequest> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("failed to set Pushgateway capture read timeout");

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        match stream.read(&mut chunk) {
            Ok(0) => return None,
            Ok(read) => {
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(index) = find_subsequence(&buffer, b"\r\n\r\n") {
                    break index + 4;
                }
            }
            Err(_) => return None,
        }
    };

    let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let content_length = headers.lines().find_map(parse_content_length).unwrap_or(0);
    while buffer.len() < header_end + content_length {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let body_end = (header_end + content_length).min(buffer.len());
    let body = String::from_utf8_lossy(&buffer[header_end..body_end]).to_string();
    let request_line = headers.lines().next().unwrap_or_default().to_string();
    let _ = stream.write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n");
    Some(CapturedHttpRequest { request_line, body })
}

fn parse_content_length(line: &str) -> Option<usize> {
    let (name, value) = line.split_once(':')?;
    name.eq_ignore_ascii_case("content-length")
        .then(|| value.trim().parse::<usize>().ok())
        .flatten()
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn temp_metrics_path(extension: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "clockping-metrics-{}-{nonce}.{extension}",
        std::process::id()
    ))
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
    let lines = output_lines(output);
    assert_eq!(lines.len(), 1, "expected exactly one JSON line: {output}");
    serde_json::from_str(lines[0]).expect("invalid JSON output")
}

fn json_lines(output: &str) -> Vec<Value> {
    output_lines(output)
        .into_iter()
        .map(|line| serde_json::from_str(line).expect("invalid JSON output"))
        .collect()
}

fn assert_timestamped_target_down_events(output: &str, protocol: &str) {
    let events = json_lines(output);
    assert_eq!(
        events.len(),
        4,
        "clockping should keep emitting events after the target goes down: {output}"
    );
    assert_eq!(events[0]["protocol"], protocol);
    assert_eq!(events[0]["status"], "reply");
    assert_eq!(events[0]["ts"], "STAMP");

    for (expected_seq, event) in events.iter().enumerate().skip(1) {
        assert_eq!(event["seq"], expected_seq, "unexpected down seq: {output}");
        assert!(
            event["status"] == "error" || event["status"] == "timeout",
            "probe after target down should report unavailable: {output}"
        );
        assert_eq!(
            event["ts"], "STAMP",
            "down events should carry clockping timestamps: {output}"
        );
    }
}

fn output_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect()
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

fn find_python3() -> Option<&'static str> {
    ["python3", "/usr/bin/python3"].into_iter().find(|program| {
        Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}
