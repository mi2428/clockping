#!/usr/bin/env bash
set -Eeuo pipefail

readonly SEMVER_TAG_RE='^v[0-9]+[.][0-9]+[.][0-9]+(-[0-9A-Za-z][0-9A-Za-z.-]*)?([+][0-9A-Za-z][0-9A-Za-z.-]*)?$'

release_created_tag=0
release_pushed_created_tag=0
release_tag=

fail() {
  echo "release: $*" >&2
  exit 1
}

run() {
  printf '+'
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

manifest_value_at_ref() {
  local ref="$1"
  local key="$2"

  git show "${ref}:Cargo.toml" | sed -n "s/^${key} = \"\\(.*\\)\"/\\1/p" | head -n 1
}

require_clean_worktree() {
  local status

  status="$(git status --porcelain)"
  if [[ -n "${status}" ]]; then
    git status --short >&2
    fail "working tree must be clean before release"
  fi
}

cleanup() {
  local status=$?

  if [[ "${release_created_tag}" == "1" && "${release_pushed_created_tag}" == "0" && -n "${release_tag}" ]]; then
    git tag -d "${release_tag}" >/dev/null 2>&1 || true
  fi

  return "${status}"
}

main() {
  local tag="${TAG:-}"
  local remote="${GIT_REMOTE:-origin}"
  local local_oid remote_line remote_oid release_ref
  local package_name package_version tag_version

  [[ -n "${tag}" ]] || fail "TAG is required, for example: make release TAG=v1.0.0"
  [[ "${tag}" =~ ${SEMVER_TAG_RE} ]] || fail "TAG must look like vMAJOR.MINOR.PATCH"

  cd "$(git rev-parse --show-toplevel)"
  release_tag="${tag}"
  trap cleanup EXIT

  require_clean_worktree

  remote_line="$(git ls-remote --tags "${remote}" "refs/tags/${tag}" | sed -n '1p')"
  remote_oid="${remote_line%%[[:space:]]*}"

  if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
    local_oid="$(git rev-parse "refs/tags/${tag}")"
    if [[ -n "${remote_oid}" && "${remote_oid}" != "${local_oid}" ]]; then
      fail "local tag ${tag} does not match ${remote}/tags/${tag}"
    fi
    printf 'Using existing tag %s at %s\n' "${tag}" "$(git rev-list -n 1 "${tag}")"
  elif [[ -n "${remote_oid}" ]]; then
    run git fetch "${remote}" "refs/tags/${tag}:refs/tags/${tag}"
    printf 'Using fetched tag %s at %s\n' "${tag}" "$(git rev-list -n 1 "${tag}")"
  else
    run git tag "${tag}"
    release_created_tag=1
    printf 'Created tag %s at %s\n' "${tag}" "$(git rev-parse HEAD)"
  fi

  release_ref="refs/tags/${tag}"
  tag_version="${tag#v}"
  package_name="$(manifest_value_at_ref "${release_ref}" "name")"
  package_version="$(manifest_value_at_ref "${release_ref}" "version")"

  [[ "${package_name}" == "clockping" ]] || fail "Cargo.toml package name is ${package_name}, expected clockping"
  [[ "${package_version}" == "${tag_version}" ]] || fail "Cargo.toml version ${package_version} does not match ${tag}"

  run git push "${remote}" "refs/tags/${tag}"
  release_pushed_created_tag=1

  printf 'Pushed release tag %s to %s. GitHub Actions will build release assets and Homebrew formula.\n' "${tag}" "${remote}"
}

main "$@"
