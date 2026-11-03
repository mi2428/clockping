"""Offline release-policy checks; no real tags, remotes, or publications are changed."""

import os
from pathlib import Path
import subprocess
import tempfile
import textwrap

helper = Path(__file__).with_name("release_metadata.sh")
with tempfile.TemporaryDirectory(prefix="clockping-release-policy-") as directory:
    root = Path(directory)
    env = dict(os.environ, GITHUB_REPOSITORY="fixture/clockping",
               GIT_AUTHOR_NAME="Fixture", GIT_AUTHOR_EMAIL="fixture@example.invalid",
               GIT_COMMITTER_NAME="Fixture", GIT_COMMITTER_EMAIL="fixture@example.invalid")

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=root, env=env, text=True).strip()

    def run(tag, success, **extra):
        output = root / "output"
        output.unlink(missing_ok=True)
        result = subprocess.run(["bash", str(helper)], cwd=root, text=True, capture_output=True,
                                env=dict(env, RELEASE_TAG=tag, GITHUB_OUTPUT=str(output), **extra))
        assert (result.returncode == 0) == success, (tag, result.stderr)
        if not success:
            assert not output.exists(), tag
        return output.read_text() if success else ""

    git("init", "-b", "main")
    manifest = root / "Cargo.toml"
    manifest.write_text('[package]\nname = "clockping"\nversion = "1.2.3"\n')
    git("add", "Cargo.toml")
    git("commit", "-m", "fixture")
    git("update-ref", "refs/remotes/origin/main", "HEAD")
    git("tag", "v1.2.3")
    assert "ghcr.io/fixture/clockping:latest\n" in run("v1.2.3", True)
    assert "ghcr.io/fixture/clockping:latest\n" not in run("v1.2.3", True, RELEASE_PRERELEASE="true")
    for tag in ("latest", "v01.2.3", "v1.2.3+build", "v1.2.4", "v1.2.3\nother"):
        run(tag, False)
    run("v1.2.3", False, RELEASE_PRERELEASE="invalid")
    manifest.write_text(manifest.read_text().replace("1.2.3", "1.2.4-rc.1"))
    git("add", "Cargo.toml")
    git("commit", "-m", "unmerged prerelease")
    git("tag", "v1.2.4-rc.1")
    run("v1.2.4-rc.1", False)
    run("v1.2.3", False)  # An old tag cannot publish the current checkout.
    git("update-ref", "refs/remotes/origin/main", "HEAD")
    prerelease = run("v1.2.4-rc.1", True)
    assert "prerelease=true\n" in prerelease and ":latest\n" not in prerelease
    manifest.write_text(manifest.read_text() + "# dirty\n")
    run("v1.2.4-rc.1", False)
    # Exercise the workflow's final shell block with a mock gh, never the real API.
    workflow = helper.parent.parent / "workflows/checks.yml"
    publish = textwrap.dedent(workflow.read_text().split("- name: Publish Linux GitHub Release assets", 1)[1].split("run: |\n", 1)[1])
    mock = root / "gh"
    mock.write_text('''#!/bin/sh
printf '%s\n' "$*" >> "$MOCK_LOG"
case "$1 $2" in
  'api --paginate')
    test "$MODE" != outage || exit 1
    if test "$MODE" != new; then printf '%s\n' "$RELEASE_TAG"; fi ;;
  'release view') printf '{"isDraft":%s,"isPrerelease":false}\n' "$DRAFT" ;;
  'release create'|'release upload') ;;
  *) exit 97 ;;
esac
''')
    mock.chmod(0o755)
    (root / "dist").mkdir()
    (root / "dist/clockping-v1.2.3-linux-amd64").write_text("fixture")
    log = root / "gh-calls"
    for mode, prerelease, draft, success in (
        ("new", "false", "false", True), ("new", "true", "false", True),
        ("existing", "false", "false", True), ("existing", "true", "false", False),
        ("existing", "false", "true", False), ("outage", "false", "false", False),
    ):
        log.write_text("")
        result = subprocess.run(["bash", "-euo", "pipefail", "-c", publish], cwd=root,
                                env=dict(env, PATH=f"{root}:{env['PATH']}", MODE=mode,
                                         RELEASE_TAG="v1.2.3", PRERELEASE=prerelease,
                                         DRAFT=draft, MOCK_LOG=str(log), GH_TOKEN="fixture"),
                                capture_output=True, text=True)
        assert (result.returncode == 0) == success, (mode, result.stderr)
        calls = log.read_text()
        if not success:
            assert "release create" not in calls and "release upload" not in calls, calls
        elif mode == "new":
            assert "release create" in calls and "--verify-tag" in calls
            assert ("--prerelease" in calls) == (prerelease == "true")
        else:
            assert "release upload" in calls and "--clobber" in calls
print("Release policy: tag/version/commit/main, prereleases, retries, and API failures passed")
