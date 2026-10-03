"""Real Fish regression: python3 tests/completion_fish.py [completion-script]."""

from pathlib import Path
import subprocess
import sys


root = Path(__file__).resolve().parents[1]
script = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "completions/clockping.fish"
commands = {"icmp", "tcp", "http", "gtp", "completion", "help"}
globals = {
    "--ts.preset", "--ts.format", "--out.format", "--out.colored",
    "--push.url", "--push.delete-on-exit", "--push.interval", "--push.job",
    "--push.label", "--push.retries", "--push.timeout", "--push.user-agent",
    "--metrics.file", "--metrics.format", "--metrics.label", "--metrics.prefix",
}
checks = 0


def candidates(line):
    global checks
    result = subprocess.run(
        ["fish", "--no-config", "-c", "source $argv[1]; complete -C $argv[2]", str(script), line],
        text=True, capture_output=True, check=True,
    )
    assert not result.stderr, (line, result.stderr)
    checks += 1
    return {item.split("\t", 1)[0] for item in result.stdout.splitlines()}


def expect(line, wanted, absent=()):
    actual = candidates(line)
    assert set(wanted) <= actual, (line, set(wanted) - actual, actual)
    assert not set(absent) & actual, (line, set(absent) & actual, actual)


expect("clockping ", commands)
expect("clockping gtp ", {"v1u", "v1c", "v2c", "help"}, commands - {"help"})
expect("clockping completion ", {"bash", "elvish", "fish", "powershell", "zsh"}, commands)
expect("clockping help ", commands)
expect("clockping help gtp ", {"v1u", "v1c", "v2c"})

for mode in ("", "icmp ", "tcp ", "http ", "gtp ", "gtp v1u ", "gtp v1c ", "gtp v2c ", "completion "):
    expect(f"clockping {mode}--", globals | {"--help", "--version"})
    expect(f"clockping {mode}-", {"-h", "-V"})
    expect(f"clockping {mode}--ts.preset ", {"local", "rfc3339", "unix", "unix-ms", "none"}, commands)
    expect(f"clockping {mode}--ts.preset un", {"unix", "unix-ms"})
    expect(f"clockping {mode}--ts.preset=un", {"--ts.preset=unix", "--ts.preset=unix-ms"})
    expect(f"clockping {mode}--out.format ", {"text", "json"})
    expect(f"clockping {mode}--out.format j", {"json"})
    expect(f"clockping {mode}--out.format=j", {"--out.format=json"})

for option in globals - {"--out.colored", "--push.delete-on-exit"}:
    for value in ("tcp", "gtp", "--gtp"):
        expect(f"clockping {option} {value} ", commands)
        expect(f"clockping {option}={value} ", commands)
        expect(f"clockping {option} {value} gtp ", {"v1u", "v1c", "v2c"}, {"tcp", "http"})

expect("clockping --push.job gtp tcp --", {"--count", "--interval", "--timeout", "--deadline"}, {"--port"})
expect("clockping gtp --push.job v1u ", {"v1u", "v1c", "v2c"})
expect("clockping gtp --push.job=v1u ", {"v1u", "v1c", "v2c"})
expect("clockping gtp --push.job tcp v2c --", {"--port", "--count"})
expect("clockping tcp --push.job gtp --", {"--count", "--quiet"}, {"--port"})
expect("clockping tcp host:80 --ts.preset un", {"unix", "unix-ms"})
expect("clockping http http://host --out.format=j", {"--out.format=json"})
expect("clockping gtp v1u host --", {"--port"} | globals)
expect("clockping tcp -", {"-4", "-6", "-c", "-i", "-W", "-w", "-q", "-h", "-V"})
expect("clockping http -", {"-4", "-6", "-c", "-i", "-W", "-w", "-X", "-H", "-L", "-k", "-q", "-h", "-V"})
for variant in ("v1u", "v1c", "v2c"):
    expect(f"clockping gtp {variant} -", {"-c", "-i", "-W", "-w", "-q", "-h", "-V"})
for option in ("--method ", "--method=", "-X ", "-X"):
    prefix = option if option.endswith("=") or option == "-X" else ""
    expect(f"clockping http {option}g", {prefix + "get"})
expect("clockping http -X ", {"head", "get"})
for option in ("--header tcp", "--header=tcp", "-H tcp", "-Htcp", "-qHtcp", "-qH tcp"):
    expect(f"clockping http {option} --", {"--method", "--location"}, {"--port"})
for option in ("--count 1", "--count=1", "-c 1", "-c1", "-qc1", "-qc 1"):
    expect(f"clockping gtp v1u {option} --", {"--port", "--count"})
for line in ("clockping -- ", "clockping tcp -- ", "clockping gtp -- ", "clockping gtp v1u -- "):
    expect(line, (), commands | globals | {"v1u", "v1c", "v2c"})
expect("clockping --push.job -- gtp ", {"v1u", "v1c", "v2c"})
expect('clockping --push.job "gtp tcp" gtp ', {"v1u", "v1c", "v2c"})
expect("clockping --ts.format '%H:%M tcp' tcp --", {"--count", "--quiet"}, {"--port"})
expect("clockping --push.job ", (), commands)
expect("clockping gtp --push.job ", (), {"v1u", "v1c", "v2c"})
expect("clockping completion --push.job ", (), {"bash", "fish", "zsh"})
expect("clockping completion fish ", (), {"bash", "elvish", "fish", "powershell", "zsh"})
expect("clockping icmp --pinger ping -c 1 --", globals)
source = script.read_text()
assert not any(obsolete in source for obsolete in (
    "__fish_clockping_global_optspecs", "__fish_clockping_needs_command",
    "__fish_clockping_using_subcommand", "argparse -s", "2>/dev/null",
)), "obsolete argparse helper path remains"
print(f"Fish completion: {checks} positive/boundary hook checks passed")
