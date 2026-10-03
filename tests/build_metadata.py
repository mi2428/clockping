"""python3 tests/build_metadata.py [--characterize]; disposable Git/Cargo fixtures only."""

from datetime import datetime, timezone
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


root = Path(__file__).resolve().parents[1]
characterize = sys.argv[1:] == ["--characterize"]
env = {key: value for key, value in os.environ.items()
       if not key.startswith("CLOCKPING_") and key not in ("SOURCE_DATE_EPOCH", "CARGO_TARGET_DIR")}
env.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1", GIT_OPTIONAL_LOCKS="0",
           GIT_AUTHOR_NAME="Fixture", GIT_COMMITTER_NAME="Fixture",
           GIT_AUTHOR_EMAIL="fixture@example.invalid", GIT_COMMITTER_EMAIL="fixture@example.invalid",
           GIT_AUTHOR_DATE="2000-01-01T00:00:00Z", GIT_COMMITTER_DATE="2000-01-01T00:00:00Z",
           SOURCE_DATE_EPOCH="0")
failures = []


def command(directory, *args, extra_env=None):
    return subprocess.run(args, cwd=directory, env=dict(env, **(extra_env or {})),
                          text=True, capture_output=True, check=True)


def git(directory, *args):
    return command(directory, "git", *args).stdout.strip()


def verify(directory, label, *, overrides=None, reuse=False):
    values = dict(env, **(overrides or {}))
    result = command(directory, "cargo", "build", "-vv", extra_env={
        **(overrides or {}), "CARGO_TARGET_DIR": str(directory / "target"),
    })
    actual = command(directory, str(directory / "target/debug/clockping"), "--version").stdout.strip()
    describe = values.get("CLOCKPING_GIT_DESCRIBE") or git(directory, "describe", "--tags", "--always", "--dirty=-dirty")
    commit = values.get("CLOCKPING_GIT_COMMIT") or git(directory, "rev-parse", "HEAD")
    date = values.get("CLOCKPING_GIT_COMMIT_DATE") or git(directory, "show", "-s", "--format=%cI", "HEAD")
    built = values.get("CLOCKPING_BUILD_DATE") or datetime.fromtimestamp(
        int(values["SOURCE_DATE_EPOCH"]), timezone.utc,
    ).strftime("%Y-%m-%dT%H:%M:%SZ")
    expected = f"clockping 0.1.0 (git {describe}; commit {commit}; commit date {date}; built {built}; debug) on "
    ok = actual.startswith(expected) and " (host " in actual
    cached = "Compiling clockping " not in result.stderr
    if not ok or (reuse and not cached):
        failures.append(label)
        if reuse and not cached:
            print("\n".join(line for line in result.stderr.splitlines() if "Dirty " in line))
    print(f"{label}: metadata={'fresh' if ok else 'STALE'}; cargo={'reused' if cached else 'rebuilt'}")
    if not ok:
        print(f"  expected: {expected}\n  actual:   {actual}")


(root / "target").mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="clockping-build-metadata-", dir=root / "target") as temporary:
    fixture = Path(temporary) / "normal"
    fixture.mkdir()
    (fixture / "src").mkdir()
    (fixture / "Cargo.toml").write_text('[package]\nname="clockping"\nversion="0.1.0"\nedition="2024"\n')
    (fixture / ".gitignore").write_text("/target/\n")
    (fixture / "state.txt").write_text("clean\n")
    (fixture / "src/main.rs").write_text('mod version;\nfn main() { println!("clockping {}", version::LONG_VERSION); }\n')
    shutil.copyfile(root / "src/version.rs", fixture / "src/version.rs")
    shutil.copyfile(root / "build.rs", fixture / "build.rs")
    git(fixture, "init")
    git(fixture, "symbolic-ref", "HEAD", "refs/heads/main")
    git(fixture, "add", ".")
    git(fixture, "commit", "-m", "fixture")
    verify(fixture, "normal initial")
    # git describe --dirty can refresh a newly created index once; not every build.
    verify(fixture, "normal index-refresh check")
    verify(fixture, "normal no-op", reuse=True)
    git(fixture, "tag", "fixture-v1")
    verify(fixture, "normal tag-only")
    (fixture / "state.txt").write_text("dirty\n")
    verify(fixture, "normal tracked-dirty-only")
    verify(fixture, "normal dirty no-op", reuse=True)
    (fixture / "state.txt").unlink()
    verify(fixture, "normal tracked deletion")
    git(fixture, "restore", "state.txt")
    verify(fixture, "normal restored-clean")
    (fixture / "added.txt").write_text("staged addition\n")
    git(fixture, "add", "added.txt")
    verify(fixture, "normal staged addition")
    git(fixture, "reset", "--", "added.txt")
    verify(fixture, "normal untracked file ignored")
    (fixture / "state.txt").write_text("new commit\n")
    git(fixture, "add", "state.txt")
    git(fixture, "commit", "-m", "advance normal branch")
    verify(fixture, "normal branch advance")
    git(fixture, "pack-refs", "--all", "--prune")
    verify(fixture, "normal packed refs")
    verify(fixture, "normal packed no-op", reuse=True)
    git(fixture, "tag", "-d", "fixture-v1")
    verify(fixture, "normal packed tag deletion")

    linked = Path(temporary) / "linked"
    git(fixture, "worktree", "add", "-b", "linked", str(linked))
    verify(linked, "linked initial")
    verify(linked, "linked index-refresh check")
    verify(linked, "linked no-op", reuse=True)
    (linked / "state.txt").write_text("linked commit\n")
    git(linked, "add", "state.txt")
    git(linked, "commit", "-m", "advance linked branch")
    verify(linked, "linked common-dir branch advance")
    git(linked, "checkout", "--detach")
    verify(linked, "linked detached HEAD")
    git(linked, "tag", "fixture-v2")
    verify(linked, "linked detached tag-only")
    (linked / "state.txt").write_text("linked dirty\n")
    verify(linked, "linked detached tracked-dirty-only")
    git(linked, "restore", "state.txt")
    verify(linked, "linked detached restored-clean")
    git(linked, "pack-refs", "--all", "--prune")
    verify(linked, "linked common-dir packed refs")
    git(linked, "tag", "-d", "fixture-v2")
    verify(linked, "linked common-dir packed tag deletion")
    verify(linked, "SOURCE_DATE_EPOCH change", overrides={"SOURCE_DATE_EPOCH": "946684800"})
    verify(linked, "explicit overrides", overrides={
        "CLOCKPING_GIT_DESCRIBE": "override-describe", "CLOCKPING_GIT_COMMIT": "override-commit",
        "CLOCKPING_GIT_COMMIT_DATE": "override-date", "CLOCKPING_BUILD_DATE": "override-build",
        "SOURCE_DATE_EPOCH": "946684800",
    })
    verify(linked, "overrides removed")

if failures and not characterize:
    raise AssertionError(f"metadata freshness/cache regressions: {failures}")
print(f"build metadata: {len(failures)} discrepancies" + (" (characterization mode)" if characterize else ""))
