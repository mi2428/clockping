use std::{fs, time::Duration};

use serde_json::Value;

use super::helpers::*;

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
fn failed_later_target_initialization_preserves_existing_metrics_file() {
    for format in ["jsonl", "prometheus"] {
        let path = temp_metrics_path(format);
        fs::write(&path, "previous metrics\n").unwrap();
        let path_arg = path.to_string_lossy();
        let result = run_clockping_raw(&[
            "--metrics.file",
            &path_arg,
            "--metrics.format",
            format,
            "tcp",
            "-6",
            "-c",
            "1",
            "[::1]:1",
            "127.0.0.1:1",
        ]);
        assert!(!result.status.success(), "invalid later target must fail");
        assert_contains(&combined_output(&result), "failed to initialize TCP prober");
        assert_eq!(fs::read_to_string(&path).unwrap(), "previous metrics\n");
        fs::remove_file(path).unwrap();
    }
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
fn slash_grouping_put_and_delete_use_the_same_base64_path() {
    let target = spawn_tcp_acceptor(1);
    let (push_url, requests) = spawn_pushgateway_capture_n(2);
    run_clockping(&[
        "--push.url",
        &push_url,
        "--push.job",
        "jobs/東京",
        "--push.label",
        "site=region/site",
        "--push.delete-on-exit",
        "tcp",
        "-c",
        "1",
        "-W",
        "1",
        &target,
    ]);
    for method in ["PUT", "DELETE"] {
        let request = requests.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(
            request.request_line,
            format!(
                "{method} /metrics/job@base64/am9icy_mnbHkuqw/site@base64/cmVnaW9uL3NpdGU HTTP/1.1"
            )
        );
    }
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
