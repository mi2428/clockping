use std::{
    net::{TcpStream, ToSocketAddrs, UdpSocket},
    process::Command,
    thread,
    time::Duration,
};

use serde_json::Value;

const DEFAULT_BIN: &str = env!("CARGO_BIN_EXE_clockping");
const RETRIES: usize = 50;
const RETRY_DELAY: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_millis(200);

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
    let output = Command::new(&bin)
        .args(args)
        .output()
        .expect("failed to spawn clockping");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    eprintln!("{combined}");

    assert!(
        output.status.success(),
        "clockping failed with status {}\n{}",
        output.status,
        combined
    );
    combined
}

fn clockping_bin() -> String {
    std::env::var("CLOCKPING_BIN").unwrap_or_else(|_| DEFAULT_BIN.to_string())
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
