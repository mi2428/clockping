use std::{
    io::{self, BufRead, BufReader},
    net::{Shutdown, TcpStream, ToSocketAddrs, UdpSocket},
    process::{Child, Command, ExitStatus, Output as ProcessOutput, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

const DEFAULT_BIN: &str = env!("CARGO_BIN_EXE_clockping");
const RETRIES: usize = 50;
const RETRY_DELAY: Duration = Duration::from_millis(100);
const PROBE_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Clone, Copy)]
pub enum GtpMode {
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

pub fn wait_for_tcp(host: &str, port: u16) {
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

pub fn wait_for_gtp(mode: GtpMode, host: &str, port: u16) {
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

pub fn run_clockping(args: &[&str]) -> String {
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

pub fn run_clockping_after_first_line(
    args: &[&str],
    after_first_line: impl FnOnce(&str),
) -> String {
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
    io::Read::read_to_string(&mut reader, &mut output).expect("failed to read remaining output");
    let stderr = child_stderr(&mut child);
    eprintln!("{output}{stderr}");

    assert!(
        status.success(),
        "clockping failed with status {status}\n{output}{stderr}"
    );
    format!("{output}{stderr}")
}

#[cfg(unix)]
pub fn run_external_ping_until_sigint_stats(ping: &str, target: &str) -> String {
    let bin = clockping_bin();
    let pinger_arg = format!("--pinger={ping}");
    let mut command = Command::new(&bin);
    command
        .args(["--ts.format", "STAMP", "icmp", &pinger_arg, target])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    set_own_process_group(&mut command);
    let mut child = command.spawn().expect("failed to spawn clockping");

    let stdout = child.stdout.take().expect("missing stdout pipe");
    let mut reader = BufReader::new(stdout);
    let mut output = String::new();
    let bytes = reader
        .read_line(&mut output)
        .expect("failed to read first external ping line");
    assert_ne!(bytes, 0, "clockping exited before external ping output");

    interrupt_process_group(child.id());
    let status = wait_for_child(&mut child, Duration::from_secs(5));
    io::Read::read_to_string(&mut reader, &mut output)
        .expect("failed to read remaining external ping output");
    let stderr = child_stderr(&mut child);
    eprintln!("{output}{stderr}");

    assert!(
        status.success(),
        "expected external ping SIGINT to exit successfully, got {status}\nstdout:\n{output}\nstderr:\n{stderr}"
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

pub fn trigger_icmp_down(host: &str, port: u16) {
    let mut stream = TcpStream::connect((host, port)).expect("failed to connect to ICMP control");
    io::Write::write_all(&mut stream, b"down").expect("failed to write ICMP down command");
    stream
        .shutdown(Shutdown::Write)
        .expect("failed to close ICMP control write side");
    thread::sleep(Duration::from_millis(100));
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

#[cfg(unix)]
fn set_own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(unix)]
fn interrupt_process_group(pid: u32) {
    let process_group = -(pid as libc::pid_t);
    // SAFETY: `kill` receives a process group ID derived from a child PID and
    // does not dereference any pointers.
    let result = unsafe { libc::kill(process_group, libc::SIGINT) };
    assert_eq!(
        result,
        0,
        "failed to send SIGINT to process group {process_group}: {}",
        std::io::Error::last_os_error()
    );
}

fn child_stderr(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        io::Read::read_to_string(&mut stderr, &mut output).expect("failed to read child stderr");
    }
    output
}

pub fn single_json_line(output: &str) -> Value {
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

pub fn assert_timestamped_target_down_events(output: &str, protocol: &str) {
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

pub fn assert_external_ping_stats_are_raw(output: &str) {
    let stats_lines = output
        .lines()
        .filter(|line| {
            line.contains("statistics")
                || line.contains("transmitted")
                || line.contains("round-trip")
                || line.contains("rtt min/avg/max")
        })
        .collect::<Vec<_>>();
    assert!(
        stats_lines.len() >= 2,
        "expected external ping statistics lines\n{output}"
    );
    for line in stats_lines {
        assert!(
            !line.starts_with("STAMP "),
            "external ping statistics must not be timestamped after Ctrl-C\n{output}"
        );
    }
}

fn output_lines(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect()
}

pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected output to contain {needle:?}\n{haystack}"
    );
}

pub fn find_ping() -> &'static str {
    ["/usr/bin/ping", "/bin/ping"]
        .into_iter()
        .find(|path| std::path::Path::new(path).exists())
        .expect("ping binary not found")
}
