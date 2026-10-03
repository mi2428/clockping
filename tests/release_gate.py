"""Run with python3 tests/release_gate.py; all release-side commands are mocked."""

import os
from pathlib import Path
import subprocess
import tempfile


root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="clockping-release-gate-") as directory:
    fixture = Path(directory)
    mock = """#!/bin/sh
printf '%s %s\n' "${0##*/}" "$*" >> "$MOCK_LOG"
case "${0##*/}:$*" in
  'git:rev-parse --show-toplevel') printf '%s\n' "$MOCK_ROOT" ;;
  'git:-C . rev-parse --is-inside-work-tree') printf 'true\n' ;;
  'git:-C . status --porcelain') ;;
  *) exit 97 ;;
esac
"""
    for name in ("git", "gh", "docker", "shasum"):
        command = fixture / name
        command.write_text(mock)
        command.chmod(0o755)
    log = fixture / "calls"
    env = dict(os.environ, PATH=f"{fixture}:{os.environ['PATH']}",
               MOCK_LOG=str(log), MOCK_ROOT=str(fixture), GH_REPO="fixture/clockping")
    for gate in ("false", "true"):
        log.write_text("")
        result = subprocess.run(
            ["make", "-f", str(root / "Makefile"), "release", "TAG=v1.0.2",
             f"RELEASE_MAKE={gate}", f"CARGO={fixture / 'unused'}"],
            cwd=root, env=env, text=True, capture_output=True,
        )
        assert result.returncode != 0, result.stdout + result.stderr
        assert f"+ {gate} check" in result.stdout, result.stdout + result.stderr
        calls = log.read_text().splitlines()
        assert calls[:3] == [
            "git rev-parse --show-toplevel",
            "git -C . rev-parse --is-inside-work-tree",
            "git -C . status --porcelain",
        ], calls
        # A passing gate reaches only the first mocked remote read, which fails.
        assert calls[3:] == ([] if gate == "false" else [
            "git ls-remote --tags origin refs/tags/v1.0.2"
        ]), calls
print("release gate: failed checks stop before remote/tag/build/publish commands")
