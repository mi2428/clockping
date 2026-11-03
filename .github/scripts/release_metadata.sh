#!/usr/bin/env bash
set -Eeuo pipefail

readonly release_tag="${RELEASE_TAG:?RELEASE_TAG is required}"
release_prerelease="${RELEASE_PRERELEASE:-false}"
if [[ "$release_tag" == *-* ]]; then release_prerelease=true; fi
readonly release_prerelease
readonly output="${GITHUB_OUTPUT:-/dev/stdout}"
readonly repository="${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"

main() {
  local build_date git_commit git_commit_date git_describe image

  # A release may only execute code already accepted into main.
  python3 - <<'PY'
import os
import re
import subprocess
import tomllib

tag = os.environ["RELEASE_TAG"]
number = r"(?:0|[1-9][0-9]*)"
if len(tag) > 128 or not re.fullmatch(rf"v{number}\.{number}\.{number}(?:-[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*)?", tag):
    raise SystemExit("RELEASE_TAG must be a Docker-compatible vMAJOR.MINOR.PATCH tag")
package = tomllib.loads(open("Cargo.toml").read())["package"]
if package["name"] != "clockping" or package["version"] != tag[1:]:
    raise SystemExit("Release tag must match the clockping package version")
def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()
if git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}") != git("rev-parse", "HEAD"):
    raise SystemExit("Release tag must point to the checked-out commit")
subprocess.run(["git", "merge-base", "--is-ancestor", "HEAD", "refs/remotes/origin/main"], check=True)
subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], check=True)
PY
  [[ "$release_prerelease" == true || "$release_prerelease" == false ]]
  image="ghcr.io/${repository,,}"
  git_commit="$(git rev-parse HEAD)"
  git_commit_date="$(git show -s --format=%cI HEAD)"
  git_describe="$(git describe --tags --always --dirty=-dirty)"
  build_date="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  {
    printf 'prerelease=%s\n' "$release_prerelease"
    printf 'tags<<EOF\n'
    printf '%s:%s\n' "${image}" "${release_tag}"
    if [[ "${release_prerelease}" != "true" ]]; then
      printf '%s:latest\n' "${image}"
    fi
    printf 'EOF\n'
    printf 'source=https://github.com/%s\n' "${repository}"
    printf 'revision=%s\n' "${git_commit}"
    printf 'version=%s\n' "${release_tag}"
    printf 'git_describe=%s\n' "${git_describe}"
    printf 'git_commit=%s\n' "${git_commit}"
    printf 'git_commit_date=%s\n' "${git_commit_date}"
    printf 'build_date=%s\n' "${build_date}"
  } >> "${output}"
}

main "$@"
