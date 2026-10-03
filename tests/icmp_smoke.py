"""Controlled native ICMP check: python3 tests/icmp_smoke.py BINARY.

Linux/macOS loopback only. Required socket/interface privileges must already
exist: this check never elevates, reconfigures interfaces, or silently skips.
"""
import json
import os
from pathlib import Path
import select
import signal
import socket
import subprocess
import sys


def main():
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    if sys.platform == "darwin":
        interface = "lo0"
    elif sys.platform.startswith("linux"):
        interface = "lo"
    else:
        raise RuntimeError("native ICMP smoke requires Linux or macOS")
    assert socket.if_nametoindex(interface) > 0
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("CLOCKPING_")}

    def run(family, targets, options=(), output=()):
        result = subprocess.run(
            [binary, "--ts.preset", "none", *output, "icmp", family, "-n", "-c", "2",
             "-i", "0", "-W", "2", *options, *targets],
            env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
        return result

    def probe(family, targets, selection=()):
        result = run(family, targets, selection)
        assert result.stdout.count("2 probes transmitted, 2 replies received") == len(targets), (
            result.stdout, result.stderr)
        assert result.stdout.count("seq=0 reply") == len(targets), result.stdout
        assert result.stdout.count("seq=1 reply") == len(targets), result.stdout

    for family, host in [("-4", "127.0.0.1"), ("-6", "::1")]:
        probe(family, [host])
        probe(family, [host], ["-I", interface])
        probe(family, [host], ["-I", host])
        probe(family, [host, host])
        result = run(family, [host], ["-s", "32", "-t", "42", "-D", "-O"],
                     ["--out.format", "json"])
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        assert len(rows) == 2, rows
        for seq, row in enumerate(rows):
            assert row["status"] == "reply" and row["seq"] == seq, row
            assert row["target"] == host and row["peer"] == host, row
            assert row["bytes"] == 40 and row["rtt_ms"] >= 0, row
            assert row["ts"] and row["detail"] == [["icmp_seq", str(seq)]], row
            # IPv6 decoder preserves its existing unavailable-hop-limit value of 0.
            if family == "-6":
                assert row["ttl"] == 0, row
            else:
                ttl = row.get("ttl")
                assert ((ttl is None and sys.platform.startswith("linux")) or
                        isinstance(ttl, int) and 0 < ttl <= 255), row
        quiet_json = run(family, [host], ["-q"], ["--out.format", "json"])
        summary = json.loads(quiet_json.stdout)
        assert (summary["sent"], summary["received"], summary["lost"],
                summary["loss_pct"]) == (2, 2, 0, 0.0), summary
        quiet = run(family, [host], ["-q"])
        assert "seq=" not in quiet.stdout, quiet.stdout
        assert "2 probes transmitted, 2 replies received" in quiet.stdout, quiet.stdout

        # Signal only after a real completed event, not during process initialization.
        process = subprocess.Popen(
            [binary, "--ts.preset", "none", "icmp", family,
             "-n", "-i", "10", "-W", "2", host],
            env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            assert select.select([process.stdout], [], [], 10)[0], "no native readiness event"
            first = process.stdout.readline()
            assert "seq=0 reply" in first, first
            process.send_signal(signal.SIGINT)
            stdout, stderr = process.communicate(timeout=5)
            assert process.returncode == 0, (process.returncode, stdout, stderr)
            assert "1 probes transmitted, 1 replies received" in stdout, stdout
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=5)
        result = subprocess.run(
            [binary, "icmp", family, "-c", "1", "-W", "2",
             "-I", "clockping-invalid-interface", host],
            env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode != 0, (result.stdout, result.stderr)
        assert "unknown network interface: clockping-invalid-interface" in (
            result.stdout + result.stderr), (result.stdout, result.stderr)
    print(f"PASS: {sys.platform}/{interface}, native IPv4/IPv6, interface/source binding, "
          "invalid interface rejection, two independent same-target probers, "
          "numeric/quiet/-D/-O/size/TTL/JSON, SIGINT")


if __name__ == "__main__":
    main()
