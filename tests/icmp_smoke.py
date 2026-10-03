"""Controlled native ICMP check: python3 tests/icmp_smoke.py BINARY.

Linux/macOS loopback only. Required socket/interface privileges must already
exist: this check never elevates, reconfigures interfaces, or silently skips.
"""
import os
from pathlib import Path
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

    def probe(family, targets, selection=()):
        result = subprocess.run(
            [binary, "--ts.preset", "none", "icmp", family, "-n", "-c", "2",
             "-i", "0", "-W", "2", *selection, *targets],
            env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
        assert result.stdout.count("2 probes transmitted, 2 replies received") == len(targets), (
            result.stdout, result.stderr)
        assert result.stdout.count("seq=0 reply") == len(targets), result.stdout
        assert result.stdout.count("seq=1 reply") == len(targets), result.stdout

    for family, host in [("-4", "127.0.0.1"), ("-6", "::1")]:
        probe(family, [host])
        probe(family, [host], ["-I", interface])
        probe(family, [host], ["-I", host])
        probe(family, [host, host])
        result = subprocess.run(
            [binary, "icmp", family, "-c", "1", "-W", "2",
             "-I", "clockping-invalid-interface", host],
            env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode != 0, (result.stdout, result.stderr)
        assert "unknown network interface: clockping-invalid-interface" in (
            result.stdout + result.stderr), (result.stdout, result.stderr)
    print(f"PASS: {sys.platform}/{interface}, native IPv4/IPv6, interface/source binding, "
          "invalid interface rejection, two independent same-target probers")


if __name__ == "__main__":
    main()
