use std::{
    io,
    net::{TcpListener, TcpStream},
    process::{Child, Command, ExitStatus, Output as ProcessOutput},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_BIN: &str = env!("CARGO_BIN_EXE_clockping");

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

pub fn run_clockping_raw(args: &[&str]) -> ProcessOutput {
    Command::new(clockping_bin())
        .args(args)
        .output()
        .expect("failed to spawn clockping")
}

pub fn clockping_bin() -> String {
    std::env::var("CLOCKPING_BIN").unwrap_or_else(|_| DEFAULT_BIN.to_string())
}

pub fn combined_output(output: &ProcessOutput) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}{stderr}")
}

pub fn unreachable_tcp_target() -> &'static str {
    // TEST-NET-1 avoids the free-local-port race when Rust runs tests in parallel.
    "192.0.2.1:9"
}

pub fn spawn_tcp_acceptor(accepts: usize) -> String {
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

pub fn spawn_http_responder(accepts: usize) -> String {
    spawn_http_responder_with_delay(accepts, Duration::ZERO)
}

pub fn spawn_delayed_http_responder(accepts: usize, delay: Duration) -> String {
    spawn_http_responder_with_delay(accepts, delay)
}

fn spawn_http_responder_with_delay(accepts: usize, delay: Duration) -> String {
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
            let _ = io::Read::read(&mut stream, &mut buffer);
            thread::sleep(delay);
            let _ = io::Write::write_all(
                &mut stream,
                b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    addr.to_string()
}

#[derive(Debug)]
pub struct CapturedHttpRequest {
    pub request_line: String,
    pub body: String,
}

pub fn spawn_pushgateway_capture() -> (String, mpsc::Receiver<CapturedHttpRequest>) {
    spawn_pushgateway_capture_n(1)
}

pub fn spawn_pushgateway_capture_n(count: usize) -> (String, mpsc::Receiver<CapturedHttpRequest>) {
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
    let request = read_pushgateway_request(&mut stream)?;
    let _ = io::Write::write_all(
        &mut stream,
        b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n",
    );
    Some(request)
}

pub fn spawn_slow_pushgateway_capture(
    delay: Duration,
) -> (
    String,
    mpsc::Receiver<CapturedHttpRequest>,
    thread::JoinHandle<()>,
) {
    spawn_pushgateway_reply_capture(&[202], delay)
}

pub fn spawn_pushgateway_reply_capture(
    statuses: &[u16],
    delay: Duration,
) -> (
    String,
    mpsc::Receiver<CapturedHttpRequest>,
    thread::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let (tx, rx) = mpsc::channel();
    let statuses = statuses.to_vec();
    let task = thread::spawn(move || {
        for status in statuses {
            let started = Instant::now();
            loop {
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "capture did not receive a request"
                );
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_pushgateway_request(&mut stream)
                            .expect("complete capture request");
                        let _ = tx.send(request);
                        thread::sleep(delay);
                        let response = format!(
                            "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                        );
                        let _ = io::Write::write_all(&mut stream, response.as_bytes());
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("slow capture accept failed: {error}"),
                }
            }
        }
    });
    (format!("http://{addr}"), rx, task)
}

fn read_pushgateway_request(stream: &mut TcpStream) -> Option<CapturedHttpRequest> {
    // BSD/macOS accept can inherit nonblocking mode from the fixture listener.
    stream
        .set_nonblocking(false)
        .expect("blocking capture stream");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("failed to set Pushgateway capture read timeout");

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        match io::Read::read(stream, &mut chunk) {
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
        match io::Read::read(stream, &mut chunk) {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let body_end = (header_end + content_length).min(buffer.len());
    let body = String::from_utf8_lossy(&buffer[header_end..body_end]).to_string();
    let request_line = headers.lines().next().unwrap_or_default().to_string();
    Some(CapturedHttpRequest { request_line, body })
}

#[test]
fn capture_waits_for_headers_and_body_on_nonblocking_listener() {
    let (url, requests, server) = spawn_pushgateway_reply_capture(&[202], Duration::ZERO);
    let mut stream = TcpStream::connect(url.strip_prefix("http://").unwrap()).unwrap();
    thread::sleep(Duration::from_millis(50));
    io::Write::write_all(
        &mut stream,
        b"PUT /metrics/job/test HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\n\r\n",
    )
    .unwrap();
    thread::sleep(Duration::from_millis(50));
    io::Write::write_all(&mut stream, b"test").unwrap();
    assert_eq!(
        requests.recv_timeout(Duration::from_secs(3)).unwrap().body,
        "test"
    );
    server.join().unwrap();
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

pub fn temp_metrics_path(extension: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "clockping-metrics-{}-{nonce}.{extension}",
        std::process::id()
    ))
}

pub fn wait_for_child(child: &mut Child, timeout: Duration) -> ExitStatus {
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
pub fn set_own_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(unix)]
pub fn interrupt_process_group(pid: u32) {
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

pub fn child_stdout(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        io::Read::read_to_string(&mut stdout, &mut output).expect("failed to read child stdout");
    }
    output
}

pub fn child_stderr(child: &mut Child) -> String {
    let mut output = String::new();
    if let Some(mut stderr) = child.stderr.take() {
        io::Read::read_to_string(&mut stderr, &mut output).expect("failed to read child stderr");
    }
    output
}

pub fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected output to contain {needle:?}\n{haystack}"
    );
}

pub fn find_python3() -> Option<&'static str> {
    ["python3", "/usr/bin/python3"].into_iter().find(|program| {
        Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    })
}
