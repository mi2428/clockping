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

pub fn unused_local_tcp_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind local TCP listener");
    let addr = listener.local_addr().expect("failed to read local address");
    drop(listener);
    addr.to_string()
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
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("failed to set Pushgateway capture read timeout");

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        match io::Read::read(&mut stream, &mut chunk) {
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
        match io::Read::read(&mut stream, &mut chunk) {
            Ok(0) => break,
            Ok(read) => buffer.extend_from_slice(&chunk[..read]),
            Err(_) => break,
        }
    }

    let body_end = (header_end + content_length).min(buffer.len());
    let body = String::from_utf8_lossy(&buffer[header_end..body_end]).to_string();
    let request_line = headers.lines().next().unwrap_or_default().to_string();
    let _ = io::Write::write_all(
        &mut stream,
        b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n",
    );
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
