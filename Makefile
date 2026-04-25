SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c

RUSTUP       ?= rustup
RUSTUP_TOOLCHAIN ?= stable
CARGO        := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) cargo >/dev/null 2>&1; then $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) cargo; else command -v cargo; fi)
RUSTC        := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) rustc >/dev/null 2>&1; then $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) rustc; else command -v rustc; fi)
RUSTDOC      := $(shell if command -v $(RUSTUP) >/dev/null 2>&1 && $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) rustdoc >/dev/null 2>&1; then $(RUSTUP) which --toolchain $(RUSTUP_TOOLCHAIN) rustdoc; else command -v rustdoc; fi)
CARGO_ENV    := RUSTC="$(RUSTC)" RUSTDOC="$(RUSTDOC)"

APP          := clockping
GH           ?= gh
GIT          ?= git
REMOTE       ?= origin
MAIN_BRANCH  ?= main
DOCKER       ?= docker
COMPOSE      ?= docker compose
LINUX_BUILD_IMAGE ?= rust:1-bookworm
DOCKER_UID   ?= $(shell id -u)
DOCKER_GID   ?= $(shell id -g)

BINDIR       := bin
DISTDIR      := dist
LINUX_CARGO_HOME_DIR := target/cargo-linux
LINUX_HOME_DIR := target/home-linux
OS           ?= darwin,linux
ARCH         ?= amd64,arm64
DARWIN_ARCHS ?= amd64 arm64
LINUX_ARCHS  ?= amd64 arm64
RUST_TARGETS := x86_64-apple-darwin aarch64-apple-darwin
MAIN_REMOTE_REF := refs/remotes/$(REMOTE)/$(MAIN_BRANCH)

DARWIN_amd64_TARGET := x86_64-apple-darwin
DARWIN_amd64_SUFFIX := darwin-amd64
DARWIN_arm64_TARGET := aarch64-apple-darwin
DARWIN_arm64_SUFFIX := darwin-arm64

LINUX_amd64_PLATFORM := linux/amd64
LINUX_amd64_SUFFIX := linux-amd64
LINUX_arm64_PLATFORM := linux/arm64
LINUX_arm64_SUFFIX := linux-arm64

all: help

##@ Development

.PHONY: build
build: ## Build the host release binary into bin/
	@echo "Building $(APP) for the host platform"
	@mkdir -p $(BINDIR)
	@$(CARGO_ENV) $(CARGO) build --release
	@cp target/release/$(APP) $(BINDIR)/$(APP)
	@chmod +x $(BINDIR)/$(APP)
	@echo "Wrote $(BINDIR)/$(APP)"

.PHONY: fmt
fmt: ## Format the Rust sources
	@$(CARGO_ENV) $(CARGO) fmt --all

.PHONY: fmt-check
fmt-check: ## Verify formatting without changing files
	@$(CARGO_ENV) $(CARGO) fmt --all --check

.PHONY: lint
lint: ## Run clippy with warnings treated as errors
	@$(CARGO_ENV) $(CARGO) clippy --all-targets --all-features -- -D warnings

.PHONY: test
test: ## Run the unit test suite
	@$(CARGO_ENV) $(CARGO) test

.PHONY: test-e2e
test-e2e: ## Run Docker Compose end-to-end tests
	@trap '$(COMPOSE) -f docker-compose.test.yml down --remove-orphans >/dev/null 2>&1 || true' EXIT; \
	$(COMPOSE) -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from sut

.PHONY: check
check: fmt-check lint test ## Run formatting, lint, and unit tests

.PHONY: ci
ci: check test-e2e ## Run the full local CI suite, including Docker e2e tests

.PHONY: _docker-check
_docker-check:
	@command -v $(DOCKER) >/dev/null 2>&1 || { \
		echo "Docker is required for Linux cross-builds" >&2; \
		exit 1; \
	}
	@$(DOCKER) info >/dev/null 2>&1 || { \
		echo "A running Docker daemon is required for Linux cross-builds" >&2; \
		exit 1; \
	}

define TARGET_RULE
.PHONY: _target.$(1)
_target.$(1):
	@command -v $(RUSTUP) >/dev/null 2>&1 || { \
		echo "rustup is required to install cross-compilation targets" >&2; \
		exit 1; \
	}
	@$(RUSTUP) target add --toolchain $(RUSTUP_TOOLCHAIN) $(1)
endef
$(foreach target,$(RUST_TARGETS),$(eval $(call TARGET_RULE,$(target))))

define DARWIN_DIST_RULE
.PHONY: _dist.darwin.$(1)
_dist.darwin.$(1): _target.$$(DARWIN_$(1)_TARGET)
	@echo "Building $(APP) for $$(DARWIN_$(1)_TARGET)"
	@mkdir -p $(DISTDIR)
	@$(CARGO_ENV) $(CARGO) build --release --target $$(DARWIN_$(1)_TARGET)
	@cp target/$$(DARWIN_$(1)_TARGET)/release/$(APP) $(DISTDIR)/$(APP)-$$(DARWIN_$(1)_SUFFIX)
	@chmod +x $(DISTDIR)/$(APP)-$$(DARWIN_$(1)_SUFFIX)
	@echo "Wrote $(DISTDIR)/$(APP)-$$(DARWIN_$(1)_SUFFIX)"
endef
$(foreach arch,$(DARWIN_ARCHS),$(eval $(call DARWIN_DIST_RULE,$(arch))))

define LINUX_DIST_RULE
.PHONY: _dist.linux.$(1)
_dist.linux.$(1): _docker-check
	@echo "Building $(APP) for $$(LINUX_$(1)_PLATFORM) via Docker"
	@mkdir -p $(DISTDIR) $(LINUX_CARGO_HOME_DIR)/$(1) $(LINUX_HOME_DIR)/$(1)
	@$(DOCKER) run --rm \
		--platform $$(LINUX_$(1)_PLATFORM) \
		-u "$(DOCKER_UID):$(DOCKER_GID)" \
		-e HOME=/workspace/$(LINUX_HOME_DIR)/$(1) \
		-e CARGO_HOME=/workspace/$(LINUX_CARGO_HOME_DIR)/$(1) \
		-e CARGO_TARGET_DIR=/workspace/target/linux-$(1) \
		-v "$(CURDIR):/workspace" \
		-w /workspace \
		$(LINUX_BUILD_IMAGE) \
		bash -c 'export PATH="/usr/local/cargo/bin:$$$$PATH"; cargo build --locked --release && cp target/linux-$(1)/release/$(APP) dist/$(APP)-$$(LINUX_$(1)_SUFFIX) && chmod +x dist/$(APP)-$$(LINUX_$(1)_SUFFIX)'
	@echo "Wrote $(DISTDIR)/$(APP)-$$(LINUX_$(1)_SUFFIX)"
endef
$(foreach arch,$(LINUX_ARCHS),$(eval $(call LINUX_DIST_RULE,$(arch))))

##@ Distribution

.PHONY: dist
dist: ## Build release binaries into dist/. Use OS=darwin,linux and ARCH=amd64,arm64
	@rm -rf $(DISTDIR)
	@mkdir -p $(DISTDIR)
	@os_list="$(OS)"; \
	arch_list="$(ARCH)"; \
	if [ -z "$$os_list" ]; then \
		echo "OS is required. Supported values: darwin,linux" >&2; \
		exit 1; \
	fi; \
	if [ -z "$$arch_list" ]; then \
		echo "ARCH is required. Supported values: amd64,arm64" >&2; \
		exit 1; \
	fi; \
	for os in $$(printf '%s' "$$os_list" | tr ',' ' '); do \
		case "$$os" in \
			darwin|linux) ;; \
			*) echo "Unsupported OS '$$os'. Supported values: darwin,linux" >&2; exit 1 ;; \
		esac; \
	done; \
	for arch in $$(printf '%s' "$$arch_list" | tr ',' ' '); do \
		case "$$arch" in \
			amd64|arm64) ;; \
			*) echo "Unsupported ARCH '$$arch'. Supported values: amd64,arm64" >&2; exit 1 ;; \
		esac; \
	done; \
	for os in $$(printf '%s' "$$os_list" | tr ',' ' '); do \
		for arch in $$(printf '%s' "$$arch_list" | tr ',' ' '); do \
			$(MAKE) _dist.$$os.$$arch || exit $$?; \
		done; \
	done
	@cd $(DISTDIR) && if command -v shasum >/dev/null 2>&1; then \
		shasum -a 256 $(APP)-* > SHA256SUMS; \
	elif command -v sha256sum >/dev/null 2>&1; then \
		sha256sum $(APP)-* > SHA256SUMS; \
	else \
		echo "shasum or sha256sum is required to write SHA256SUMS" >&2; \
		exit 1; \
	fi
	@echo "Wrote $(DISTDIR)/SHA256SUMS"

.PHONY: clean
clean: ## Remove build artifacts
	@echo "Cleaning build artifacts"
	@rm -rf $(BINDIR) $(DISTDIR) .cargo-linux .home-linux $(LINUX_CARGO_HOME_DIR) $(LINUX_HOME_DIR)
	@$(CARGO) clean

##@ Release

.PHONY: _publish-release
_publish-release:
	@command -v $(GH) >/dev/null 2>&1 || { \
		echo "gh is required to publish a release" >&2; \
		exit 1; \
	}
	@if [ -z "$(TAG)" ]; then \
		echo "TAG is required for the release upload step" >&2; \
		exit 1; \
	fi
	@if [ -z "$(TARGET_SHA)" ]; then \
		echo "TARGET_SHA is required for the release upload step" >&2; \
		exit 1; \
	fi
	@if $(GH) release view "$(TAG)" >/dev/null 2>&1; then \
		echo "Release $(TAG) already exists" >&2; \
		exit 1; \
	fi
	@if $(GIT) ls-remote --exit-code --tags "$(REMOTE)" "refs/tags/$(TAG)" >/dev/null 2>&1; then \
		echo "Tag $(TAG) already exists on $(REMOTE)" >&2; \
		exit 1; \
	fi
	@if ! ls $(DISTDIR)/$(APP)-* >/dev/null 2>&1; then \
		echo "No release assets found in $(DISTDIR). Run make dist first." >&2; \
		exit 1; \
	fi
	@echo "Creating release $(TAG) at $(TARGET_SHA)"
	@$(GH) release create "$(TAG)" $(DISTDIR)/$(APP)-* $(DISTDIR)/SHA256SUMS \
		--target "$(TARGET_SHA)" \
		--title "$(TAG)" \
		--notes "Release $(TAG) built from $(TARGET_SHA)"

.PHONY: release
release: ## Build cross-compiled binaries for origin/main and publish a GitHub Release
	@command -v $(GIT) >/dev/null 2>&1 || { \
		echo "git is required to create a release" >&2; \
		exit 1; \
	}
	@make_bin="$$(command -v make)"; \
	tmpdir="$$(mktemp -d)"; \
	main_ref="$(MAIN_REMOTE_REF)"; \
	trap 'status=$$?; $(GIT) worktree remove --force "$$tmpdir" >/dev/null 2>&1 || true; rm -rf "$$tmpdir"; exit $$status' EXIT; \
	echo "Fetching $(REMOTE)/$(MAIN_BRANCH)"; \
	$(GIT) fetch $(REMOTE) $(MAIN_BRANCH); \
	main_sha="$$($(GIT) rev-parse "$$main_ref")"; \
	echo "Preparing worktree for $$main_sha"; \
	$(GIT) worktree add --force --detach "$$tmpdir" "$$main_sha" >/dev/null; \
	release_version="$$(awk 'BEGIN { in_pkg = 0 } /^\[package\]$$/ { in_pkg = 1; next } /^\[/ { in_pkg = 0 } in_pkg && $$1 == "version" { gsub(/"/, "", $$3); print $$3; exit }' "$$tmpdir/Cargo.toml")"; \
	if [ -z "$$release_version" ]; then \
		echo "failed to read package.version from $$tmpdir/Cargo.toml" >&2; \
		exit 1; \
	fi; \
	tag="v$$release_version"; \
	echo "Building release assets for $$tag"; \
	"$$make_bin" -f "$(CURDIR)/Makefile" -C "$$tmpdir" dist OS=darwin,linux ARCH=amd64,arm64; \
	echo "Publishing $$tag"; \
	"$$make_bin" -f "$(CURDIR)/Makefile" -C "$$tmpdir" _publish-release TAG="$$tag" TARGET_SHA="$$main_sha"

##@ Help

.PHONY: help
help: ## Show this help message
	@awk 'BEGIN {FS = ":.*##"; section = ""} \
	/^[a-zA-Z0-9_.-]+:.*##/ { \
		if (section != "") printf "\n\033[1m%s\033[0m\n", section; \
		section = ""; \
		printf "  \033[36m%-11s\033[0m %s\n", $$1, $$2; next \
	} \
	/^##@/ { section = substr($$0, 5); next }' $(MAKEFILE_LIST)
	@printf "\n\033[1mExamples:\033[0m\n"
	@printf "  \033[36mmake build\033[0m\n"
	@printf "  \033[36mmake check\033[0m\n"
	@printf "  \033[36mmake test-e2e\033[0m\n"
	@printf "  \033[36mmake dist OS=darwin ARCH=arm64\033[0m\n"
	@printf "  \033[36mmake dist OS=darwin,linux ARCH=amd64,arm64\033[0m\n"
	@printf "  \033[36mmake -n release\033[0m\n"
	@printf "  \033[36mmake release\033[0m\n"
