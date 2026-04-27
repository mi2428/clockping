use super::helpers::*;

// This Docker E2E test exercises clockping's protocol-facing paths against
// real containers on one Compose network:
// - TCP connect and HTTP HEAD use Python's http.server as the endpoint;
// - transient TCP, HTTP, ICMP, and GTP targets verify that target-down probes
//   keep producing timestamped events instead of leaving a silent terminal;
// - native ICMP and external ping verify both ICMP engines inside the network;
// - GTPv1-U, GTPv1-C, and GTPv2-C use the small Python echo responder.
// It intentionally remains one Rust test so Compose setup is paid once while
// each protocol still has an explicit output assertion below.
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
        "--ts.preset",
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
        "--ts.preset",
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
        "--ts.format",
        "STAMP",
        "--out.format",
        "json",
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
        "--ts.format",
        "STAMP",
        "--out.format",
        "json",
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
        "--ts.format",
        "STAMP",
        "--out.format",
        "json",
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
        "--ts.format",
        "STAMP",
        "--out.format",
        "json",
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
            "--ts.format",
            "STAMP",
            "--out.format",
            "json",
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
            "--ts.format",
            "STAMP",
            "--out.format",
            "json",
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
        "--ts.preset",
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
        "--ts.preset",
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

    #[cfg(unix)]
    {
        let sigint_output = run_external_ping_until_sigint_stats(find_ping(), "tcp-target");
        assert_contains(&sigint_output, "STAMP PING tcp-target");
        assert_external_ping_stats_are_raw(&sigint_output);
    }

    let gtp_v1u_output = run_clockping(&[
        "--ts.preset",
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
        "--ts.preset",
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
        "--ts.preset",
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
