//! Replay a captured clockping CUI run for README recordings.

use std::{
    env,
    io::{self, Write},
    process::ExitCode,
    thread,
    time::Duration,
};

// Captured output for:
// clockping icmp -c 4 2001:4860:4860::8888 8.8.8.8 2606:4700:4700::1111 1.1.1.1
//
// The delay is milliseconds since the previous line. Replies for the same
// sequence use zero delay to match probes completing at the same tick.
struct Frame {
    delay_ms: u64,
    timestamp: Option<&'static str>,
    line: &'static str,
}

const FRAMES: &[Frame] = &[
    Frame {
        delay_ms: 120,
        timestamp: Some("2026-05-15 13:40:41.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2606:4700:4700::1111 (2606:4700:4700::1111)\x1b[0m seq=\x1b[33m0\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2606:4700:4700::1111\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m7.546ms\x1b[0m icmp_seq=\x1b[33m0\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:41.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2001:4860:4860::8888 (2001:4860:4860::8888)\x1b[0m seq=\x1b[33m0\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2001:4860:4860::8888\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m7.538ms\x1b[0m icmp_seq=\x1b[33m0\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:41.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m8.8.8.8 (8.8.8.8)\x1b[0m seq=\x1b[33m0\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m8.8.8.8\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m117\x1b[0m rtt=\x1b[32m7.841ms\x1b[0m icmp_seq=\x1b[33m0\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:41.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m seq=\x1b[33m0\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m1.1.1.1\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m58\x1b[0m rtt=\x1b[32m7.914ms\x1b[0m icmp_seq=\x1b[33m0\x1b[0m",
    },
    Frame {
        delay_ms: 1_000,
        timestamp: Some("2026-05-15 13:40:42.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2001:4860:4860::8888 (2001:4860:4860::8888)\x1b[0m seq=\x1b[33m1\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2001:4860:4860::8888\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m5.601ms\x1b[0m icmp_seq=\x1b[33m1\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:42.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2606:4700:4700::1111 (2606:4700:4700::1111)\x1b[0m seq=\x1b[33m1\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2606:4700:4700::1111\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m5.438ms\x1b[0m icmp_seq=\x1b[33m1\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:42.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m8.8.8.8 (8.8.8.8)\x1b[0m seq=\x1b[33m1\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m8.8.8.8\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m117\x1b[0m rtt=\x1b[32m5.759ms\x1b[0m icmp_seq=\x1b[33m1\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:42.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m seq=\x1b[33m1\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m1.1.1.1\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m58\x1b[0m rtt=\x1b[32m6.860ms\x1b[0m icmp_seq=\x1b[33m1\x1b[0m",
    },
    Frame {
        delay_ms: 1_000,
        timestamp: Some("2026-05-15 13:40:43.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2001:4860:4860::8888 (2001:4860:4860::8888)\x1b[0m seq=\x1b[33m2\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2001:4860:4860::8888\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m4.787ms\x1b[0m icmp_seq=\x1b[33m2\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:43.893 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2606:4700:4700::1111 (2606:4700:4700::1111)\x1b[0m seq=\x1b[33m2\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2606:4700:4700::1111\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m4.930ms\x1b[0m icmp_seq=\x1b[33m2\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:43.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m8.8.8.8 (8.8.8.8)\x1b[0m seq=\x1b[33m2\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m8.8.8.8\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m117\x1b[0m rtt=\x1b[32m5.114ms\x1b[0m icmp_seq=\x1b[33m2\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:43.893 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m seq=\x1b[33m2\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m1.1.1.1\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m58\x1b[0m rtt=\x1b[32m5.767ms\x1b[0m icmp_seq=\x1b[33m2\x1b[0m",
    },
    Frame {
        delay_ms: 1_000,
        timestamp: Some("2026-05-15 13:40:44.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2606:4700:4700::1111 (2606:4700:4700::1111)\x1b[0m seq=\x1b[33m3\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2606:4700:4700::1111\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m5.144ms\x1b[0m icmp_seq=\x1b[33m3\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:44.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m2001:4860:4860::8888 (2001:4860:4860::8888)\x1b[0m seq=\x1b[33m3\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m2001:4860:4860::8888\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m0\x1b[0m rtt=\x1b[32m5.263ms\x1b[0m icmp_seq=\x1b[33m3\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "--- \x1b[34m2001:4860:4860::8888 (2001:4860:4860::8888)\x1b[0m \x1b[1mclockping statistics\x1b[0m ---",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "4 probes transmitted, 4 replies received, \x1b[32m0\x1b[0m lost, \x1b[32m0.0% loss\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "rtt min/avg/max = \x1b[32m4.787ms\x1b[0m/\x1b[36m5.797ms\x1b[0m/\x1b[35m7.538ms\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "--- \x1b[34m2606:4700:4700::1111 (2606:4700:4700::1111)\x1b[0m \x1b[1mclockping statistics\x1b[0m ---",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "4 probes transmitted, 4 replies received, \x1b[32m0\x1b[0m lost, \x1b[32m0.0% loss\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "rtt min/avg/max = \x1b[32m4.930ms\x1b[0m/\x1b[36m5.765ms\x1b[0m/\x1b[35m7.546ms\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:44.894 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m seq=\x1b[33m3\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m1.1.1.1\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m58\x1b[0m rtt=\x1b[32m6.284ms\x1b[0m icmp_seq=\x1b[33m3\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: Some("2026-05-15 13:40:44.895 +0900"),
        line: "\x1b[36micmp\x1b[0m \x1b[34m8.8.8.8 (8.8.8.8)\x1b[0m seq=\x1b[33m3\x1b[0m \x1b[32mreply\x1b[0m from=\x1b[34m8.8.8.8\x1b[0m bytes=\x1b[36m64\x1b[0m ttl=\x1b[35m117\x1b[0m rtt=\x1b[32m6.325ms\x1b[0m icmp_seq=\x1b[33m3\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "--- \x1b[34m1.1.1.1 (1.1.1.1)\x1b[0m \x1b[1mclockping statistics\x1b[0m ---",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "4 probes transmitted, 4 replies received, \x1b[32m0\x1b[0m lost, \x1b[32m0.0% loss\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "rtt min/avg/max = \x1b[32m5.767ms\x1b[0m/\x1b[36m6.706ms\x1b[0m/\x1b[35m7.914ms\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "--- \x1b[34m8.8.8.8 (8.8.8.8)\x1b[0m \x1b[1mclockping statistics\x1b[0m ---",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "4 probes transmitted, 4 replies received, \x1b[32m0\x1b[0m lost, \x1b[32m0.0% loss\x1b[0m",
    },
    Frame {
        delay_ms: 0,
        timestamp: None,
        line: "rtt min/avg/max = \x1b[32m5.114ms\x1b[0m/\x1b[36m6.260ms\x1b[0m/\x1b[35m7.841ms\x1b[0m",
    },
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cui_demo: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let delay_scale = demo_delay_scale();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for frame in FRAMES {
        sleep_scaled(frame.delay_ms, delay_scale);
        if let Some(timestamp) = frame.timestamp {
            write!(stdout, "\x1b[2m{timestamp}\x1b[0m ")?;
        }
        writeln!(stdout, "{}", frame.line)?;
        stdout.flush()?;
    }

    Ok(())
}

fn demo_delay_scale() -> f64 {
    env::var("CLOCKPING_DEMO_DELAY_SCALE")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|scale| scale.is_finite() && *scale >= 0.0)
        .unwrap_or(1.0)
}

fn sleep_scaled(milliseconds: u64, scale: f64) {
    let scaled = (milliseconds as f64 * scale).round();
    if scaled <= 0.0 {
        return;
    }
    thread::sleep(Duration::from_millis(scaled as u64));
}
