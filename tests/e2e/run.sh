#!/usr/bin/env bash
set -euo pipefail

bin="${CLOCKPING_BIN:-target/debug/clockping}"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

run_cmd() {
  echo "+ $*" >&2
  local output
  if ! output="$("$@" 2>&1)"; then
    echo "$output" >&2
    fail "command failed: $*"
  fi
  echo "$output" >&2
  printf "%s" "$output"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  case "$haystack" in
    *"$needle"*) ;;
    *) fail "expected output to contain: $needle" ;;
  esac
}

wait_for_tcp() {
  local host="$1"
  local port="$2"
  for _ in $(seq 1 50); do
    if python3 - "$host" "$port" >/dev/null 2>&1 <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
with socket.create_connection((host, port), timeout=0.2):
    pass
PY
    then
      return 0
    fi
    sleep 0.1
  done
  fail "tcp target did not become ready: $host:$port"
}

wait_for_gtp() {
  local mode="$1"
  local host="$2"
  local port="$3"
  for _ in $(seq 1 50); do
    if python3 - "$mode" "$host" "$port" >/dev/null 2>&1 <<'PY'
import socket
import sys

mode = sys.argv[1]
host = sys.argv[2]
port = int(sys.argv[3])
packet = (
    bytes([0x32, 0x01, 0x00, 0x04, 0, 0, 0, 0, 0x12, 0x34, 0, 0])
    if mode == "v1"
    else bytes([0x40, 0x01, 0x00, 0x04, 0x12, 0x34, 0x56, 0])
)
with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
    sock.settimeout(0.2)
    sock.sendto(packet, (host, port))
    response, _peer = sock.recvfrom(2048)
    if response[1] != 2:
        raise SystemExit(1)
PY
    then
      return 0
    fi
    sleep 0.1
  done
  fail "gtp target did not become ready: $host:$port"
}

cargo build --locked --quiet

wait_for_tcp tcp-target 8080
wait_for_gtp v1 gtp-v1u-target 2152
wait_for_gtp v1 gtp-v1c-target 2123
wait_for_gtp v2 gtp-v2c-target 2123

tcp_output="$(run_cmd "$bin" --timestamp none tcp -c 1 -W 1 tcp-target:8080)"
assert_contains "$tcp_output" "tcp tcp-target:8080 seq=0 reply"
assert_contains "$tcp_output" "1 probes transmitted, 1 replies received"

json_output="$(run_cmd "$bin" --timestamp-format STAMP --json tcp -c 1 -W 1 tcp-target:8080)"
JSON_OUTPUT="$json_output" python3 - <<'PY'
import json
import os

lines = [line for line in os.environ["JSON_OUTPUT"].splitlines() if line.strip()]
if len(lines) != 1:
    raise SystemExit(f"expected exactly one JSON line, got {len(lines)}")
event = json.loads(lines[0])
assert event["ts"] == "STAMP", event
assert event["protocol"] == "tcp", event
assert event["status"] == "reply", event
assert event["seq"] == 0, event
assert event["rtt_ms"] >= 0, event
PY

json_summary_output="$(run_cmd "$bin" --timestamp-format STAMP --json tcp -q -c 1 -W 1 tcp-target:8080)"
JSON_OUTPUT="$json_summary_output" python3 - <<'PY'
import json
import os

lines = [line for line in os.environ["JSON_OUTPUT"].splitlines() if line.strip()]
if len(lines) != 1:
    raise SystemExit(f"expected exactly one JSON summary line, got {len(lines)}")
summary = json.loads(lines[0])
assert summary["type"] == "summary", summary
assert summary["target"] == "tcp-target:8080", summary
assert summary["sent"] == 1, summary
assert summary["received"] == 1, summary
assert summary["lost"] == 0, summary
assert summary["rtt_min_ms"] >= 0, summary
PY

icmp_output="$(run_cmd "$bin" --timestamp none icmp -4 -c 1 -W 1 tcp-target)"
assert_contains "$icmp_output" "icmp tcp-target ("
assert_contains "$icmp_output" "seq=0 reply"
assert_contains "$icmp_output" "1 probes transmitted, 1 replies received"

pinger="$(command -v ping)"
wrapper_output="$(run_cmd "$bin" --timestamp none icmp "--pinger=$pinger" -c 1 -W 1 tcp-target)"
assert_contains "$wrapper_output" "PING tcp-target"
assert_contains "$wrapper_output" "1 received"

gtp_v1u_output="$(run_cmd "$bin" --timestamp none gtp v1u -c 1 -W 1 gtp-v1u-target)"
assert_contains "$gtp_v1u_output" "gtpv1u gtp-v1u-target:2152 seq=0 reply"
assert_contains "$gtp_v1u_output" "gtp_seq=0"

gtp_v1c_output="$(run_cmd "$bin" --timestamp none gtp v1c -c 1 -W 1 gtp-v1c-target)"
assert_contains "$gtp_v1c_output" "gtpv1c gtp-v1c-target:2123 seq=0 reply"
assert_contains "$gtp_v1c_output" "gtp_seq=0"

gtp_v2c_output="$(run_cmd "$bin" --timestamp none gtp v2c -c 1 -W 1 gtp-v2c-target)"
assert_contains "$gtp_v2c_output" "gtpv2c gtp-v2c-target:2123 seq=0 reply"
assert_contains "$gtp_v2c_output" "gtp_seq=0"

echo "e2e tests passed"
