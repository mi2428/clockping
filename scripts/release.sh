#!/usr/bin/env bash
set -Eeuo pipefail

readonly SEMVER_TAG_RE='^v[0-9]+[.][0-9]+[.][0-9]+(-[0-9A-Za-z][0-9A-Za-z.-]*)?([+][0-9A-Za-z][0-9A-Za-z.-]*)?$'
readonly APP=clockping
readonly HOMEBREW_DESC='A multi-protocol, multi-target pinger for watching hosts go dark'

release_created_tag=0
release_pushed_created_tag=0
release_tag=
docker_image_tags=()

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

require_tool() {
  local tool="$1"

  command -v "${tool}" >/dev/null 2>&1 || fail "${tool} is required for local release publishing"
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

repository_slug() {
  local remote="$1"
  local repo="${GH_REPO:-${GITHUB_REPOSITORY:-}}"
  local url

  if [[ -z "${repo}" ]]; then
    url="$(git config --get "remote.${remote}.url" || true)"
    case "${url}" in
      git@github.com:*) repo="${url#git@github.com:}" ;;
      https://github.com/*) repo="${url#https://github.com/}" ;;
      ssh://git@github.com/*) repo="${url#ssh://git@github.com/}" ;;
      *) fail "could not infer GitHub repository from remote ${remote}; set GH_REPO=owner/repo" ;;
    esac
  fi

  repo="${repo#https://github.com/}"
  repo="${repo%.git}"

  [[ "${repo}" == */* ]] || fail "GitHub repository must look like owner/repo, got ${repo}"
  printf '%s\n' "${repo}"
}

lowercase() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

is_prerelease_tag() {
  [[ "$1" == *-* ]]
}

release_build_date() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

require_release_tools() {
  require_tool git
  require_tool gh
  require_tool "${DOCKER:-docker}"
  require_tool shasum
}

build_dist() {
  local os="${OS:-darwin,linux}"
  local arch="${ARCH:-amd64,arm64}"

  run "${MAKE:-make}" dist OS="${os}" ARCH="${arch}"
}

ensure_buildx_builder() {
  local docker="${DOCKER:-docker}"

  if "${docker}" buildx inspect >/dev/null 2>&1; then
    return
  fi

  run "${docker}" buildx create --use --name "${APP}-release-builder"
}

build_and_push_docker_image() {
  local tag="$1"
  local repository="$2"
  local image="${DOCKER_IMAGE:-${IMAGE:-}}"
  local platforms="${DOCKER_PLATFORMS:-linux/amd64,linux/arm64}"
  local docker="${DOCKER:-docker}"
  local git_commit git_commit_date git_describe build_date image_tag
  local tag_args=()

  if [[ -z "${image}" ]]; then
    image="ghcr.io/$(lowercase "${repository}")"
  fi

  docker_image_tags=("${image}:${tag}")
  if ! is_prerelease_tag "${tag}"; then
    docker_image_tags+=("${image}:latest")
  fi

  for image_tag in "${docker_image_tags[@]}"; do
    tag_args+=(--tag "${image_tag}")
  done

  git_commit="$(git rev-parse HEAD)"
  git_commit_date="$(git show -s --format=%cI HEAD)"
  git_describe="$(git describe --tags --always --dirty=-dirty)"
  build_date="$(release_build_date)"

  ensure_buildx_builder
  run "${docker}" buildx build \
    --platform "${platforms}" \
    --target release \
    --push \
    --label "org.opencontainers.image.revision=${git_commit}" \
    --label "org.opencontainers.image.version=${tag}" \
    --build-arg "CLOCKPING_BUILD_DATE=${build_date}" \
    --build-arg "CLOCKPING_GIT_COMMIT=${git_commit}" \
    --build-arg "CLOCKPING_GIT_COMMIT_DATE=${git_commit_date}" \
    --build-arg "CLOCKPING_GIT_DESCRIBE=${git_describe}" \
    "${tag_args[@]}" \
    .
}

write_docker_image_manifest() {
  local dist_dir="${DISTDIR:-dist}"
  local image_file="${dist_dir}/docker-images.txt"
  local image_tag

  mkdir -p "${dist_dir}"
  {
    printf 'Docker images published by make release:\n'
    for image_tag in "${docker_image_tags[@]}"; do
      printf '%s\n' "${image_tag}"
    done
  } > "${image_file}"
  printf 'Wrote %s\n' "${image_file}"
}

release_assets() {
  local dist_dir="${DISTDIR:-dist}"
  local assets=()

  shopt -s nullglob
  assets=("${dist_dir}"/*)
  shopt -u nullglob

  ((${#assets[@]} > 0)) || fail "no release assets found in ${dist_dir}"
  printf '%s\0' "${assets[@]}"
}

publish_github_release() {
  local tag="$1"
  local release_commit="$2"
  local repository="$3"
  local prerelease_flag=()
  local assets=()

  while IFS= read -r -d '' asset; do
    assets+=("${asset}")
  done < <(release_assets)

  if is_prerelease_tag "${tag}"; then
    prerelease_flag=(--prerelease)
  fi

  if gh release view "${tag}" --repo "${repository}" >/dev/null 2>&1; then
    run gh release upload "${tag}" "${assets[@]}" --clobber --repo "${repository}"
    return
  fi

  run gh release create "${tag}" \
    --repo "${repository}" \
    --target "${release_commit}" \
    --title "${tag}" \
    --generate-notes \
    "${prerelease_flag[@]}" \
    "${assets[@]}"
}

homebrew_tap_enabled() {
  case "${HOMEBREW_TAP:-1}" in
    0|false|FALSE|no|NO) return 1 ;;
    *) return 0 ;;
  esac
}

require_clean_git_dir() {
  local dir="$1"
  local label="$2"
  local status

  [[ -d "${dir}/.git" ]] || fail "${label} repo not found at ${dir}; set HOMEBREW_TAP_DIR or HOMEBREW_TAP=0"

  status="$(git -C "${dir}" status --porcelain)"
  if [[ -n "${status}" ]]; then
    git -C "${dir}" status --short >&2
    fail "${label} working tree must be clean before release"
  fi
}

write_homebrew_tap_readme() {
  local tap_dir="$1"

  cat > "${tap_dir}/README.md" <<'README'
# homebrew-clockping

Homebrew tap for `clockping`.

```console
$ brew tap mi2428/clockping
$ brew install clockping
```
README
}

write_homebrew_formula() {
  local formula_file="$1"
  local tag="$2"
  local version="$3"
  local repository="$4"
  local darwin_amd64_sha="$5"
  local darwin_arm64_sha="$6"

  cat > "${formula_file}" <<FORMULA
# typed: false
# frozen_string_literal: true

class Clockping < Formula
  desc "${HOMEBREW_DESC}"
  homepage "https://github.com/${repository}"
  version "${version}"
  license "MIT"
  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/${repository}/releases/download/${tag}/${APP}-darwin-arm64",
          using: :nounzip
      sha256 "${darwin_arm64_sha}"
    end

    on_intel do
      url "https://github.com/${repository}/releases/download/${tag}/${APP}-darwin-amd64",
          using: :nounzip
      sha256 "${darwin_amd64_sha}"
    end
  end

  def install
    bin.install Dir["${APP}-darwin-*"].first => "${APP}"
    chmod 0755, bin/"${APP}"
    generate_completions_from_executable(bin/"${APP}", "completion")
  end

  test do
    assert_match "${APP} #{version}", shell_output("#{bin}/${APP} --version")
    assert_match "Usage:", shell_output("#{bin}/${APP} --help")
  end
end
FORMULA
}

publish_homebrew_formula() {
  local tag="$1"
  local repository="$2"
  local version="${tag#v}"
  local tap_dir="${HOMEBREW_TAP_DIR:-../homebrew-clockping}"
  local tap_remote="${HOMEBREW_TAP_REMOTE:-origin}"
  local dist_dir="${DISTDIR:-dist}"
  local formula_dir="${tap_dir}/Formula"
  local formula_file="${formula_dir}/${APP}.rb"
  local darwin_amd64_binary="${dist_dir}/${APP}-darwin-amd64"
  local darwin_arm64_binary="${dist_dir}/${APP}-darwin-arm64"
  local darwin_amd64_sha darwin_arm64_sha

  if ! homebrew_tap_enabled; then
    printf 'Skipping Homebrew tap update because HOMEBREW_TAP=0\n'
    return
  fi

  [[ -f "${darwin_amd64_binary}" ]] || fail "missing Homebrew artifact ${darwin_amd64_binary}; include OS=darwin ARCH=amd64"
  [[ -f "${darwin_arm64_binary}" ]] || fail "missing Homebrew artifact ${darwin_arm64_binary}; include OS=darwin ARCH=arm64"

  require_clean_git_dir "${tap_dir}" "Homebrew tap"

  darwin_amd64_sha="$(sha256_file "${darwin_amd64_binary}")"
  darwin_arm64_sha="$(sha256_file "${darwin_arm64_binary}")"

  mkdir -p "${formula_dir}"
  write_homebrew_tap_readme "${tap_dir}"
  write_homebrew_formula "${formula_file}" "${tag}" "${version}" "${repository}" "${darwin_amd64_sha}" "${darwin_arm64_sha}"

  if command -v brew >/dev/null 2>&1; then
    run env HOMEBREW_DEVELOPER=1 brew style --except-cops FormulaAudit/Homepage,FormulaAudit/Desc,FormulaAuditStrict --fix "${formula_file}" || true
  else
    printf 'Skipping Homebrew style; brew not found\n'
  fi

  run git -C "${tap_dir}" add README.md "Formula/${APP}.rb"

  if git -C "${tap_dir}" diff --cached --quiet; then
    printf 'Homebrew formula is already up to date for %s\n' "${tag}"
    return
  fi

  run git -C "${tap_dir}" commit -m "${APP} ${version}"
  run git -C "${tap_dir}" push "${tap_remote}" HEAD
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
  local local_oid remote_line remote_oid release_ref release_commit head_commit repository
  local package_name package_version tag_version

  [[ -n "${tag}" ]] || fail "TAG is required, for example: make release TAG=v1.0.0"
  [[ "${tag}" =~ ${SEMVER_TAG_RE} ]] || fail "TAG must look like vMAJOR.MINOR.PATCH"

  cd "$(git rev-parse --show-toplevel)"
  release_tag="${tag}"
  trap cleanup EXIT

  require_clean_worktree
  require_release_tools
  repository="$(repository_slug "${remote}")"

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
  release_commit="$(git rev-list -n 1 "${tag}")"
  head_commit="$(git rev-parse HEAD)"
  [[ "${release_commit}" == "${head_commit}" ]] || fail "${tag} points to ${release_commit}, but HEAD is ${head_commit}; checkout the release commit first"

  tag_version="${tag#v}"
  package_name="$(manifest_value_at_ref "${release_ref}" "name")"
  package_version="$(manifest_value_at_ref "${release_ref}" "version")"

  [[ "${package_name}" == "${APP}" ]] || fail "Cargo.toml package name is ${package_name}, expected ${APP}"
  [[ "${package_version}" == "${tag_version}" ]] || fail "Cargo.toml version ${package_version} does not match ${tag}"

  build_dist
  build_and_push_docker_image "${tag}" "${repository}"
  write_docker_image_manifest

  run git push "${remote}" "refs/tags/${tag}"
  release_pushed_created_tag=1

  publish_github_release "${tag}" "${release_commit}" "${repository}"
  publish_homebrew_formula "${tag}" "${repository}"

  printf 'Published %s from local artifacts and Docker image(s).\n' "${tag}"
}

main "$@"
