# syntax=docker/dockerfile:1
ARG BUILD_IMAGE=rust
ARG BUILD_IMAGE_TAG=1-bookworm
ARG RUNTIME_IMAGE=debian
ARG RUNTIME_IMAGE_TAG=bookworm-slim
ARG RELEASE_BUILD_IMAGE=rust
ARG RELEASE_BUILD_IMAGE_TAG=1-alpine

FROM ${BUILD_IMAGE}:${BUILD_IMAGE_TAG} AS integration-build

WORKDIR /workspace
COPY . .

RUN --mount=type=cache,id=clockping-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=clockping-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=clockping-integration-target,target=/workspace/target,sharing=locked \
    rm -f target/debug/deps/integration_test-* \
 && cargo build --locked \
 && cargo test --locked --test integration_test --no-run \
 && mkdir -p /out \
 && install -m 0755 target/debug/clockping /out/clockping \
 && test_bin="$(find target/debug/deps -maxdepth 1 -type f -name 'integration_test-*' -perm /111 | head -n 1)" \
 && test -n "$test_bin" \
 && install -m 0755 "$test_bin" /out/clockping-integration-test

FROM ${RUNTIME_IMAGE}:${RUNTIME_IMAGE_TAG} AS integration-test

# hadolint ignore=DL3008
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
      ca-certificates \
      iproute2 \
      iputils-ping \
      python3 \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /work
COPY --from=integration-build /out/clockping /usr/local/bin/clockping
COPY --from=integration-build /out/clockping-integration-test /usr/local/bin/clockping-integration-test
COPY tests/e2e/gtp_echo_server.py tests/e2e/gtp_echo_server.py

ENV CLOCKPING_BIN=/usr/local/bin/clockping
CMD ["/usr/local/bin/clockping-integration-test", "--ignored", "--nocapture"]

FROM ${RELEASE_BUILD_IMAGE}:${RELEASE_BUILD_IMAGE_TAG} AS release-build

# Keep the scratch release image independent of an OS CA bundle. clockping
# currently has no TLS client; if one is added, use embedded Rustls/webpki roots
# rather than copying /etc/ssl/certs into the final image.
# hadolint ignore=DL3018
RUN apk add --no-cache build-base linux-headers

WORKDIR /workspace
COPY . .

RUN --mount=type=cache,id=clockping-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=clockping-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=clockping-release-target,target=/workspace/target,sharing=locked \
    cargo build --release --locked \
 && mkdir -p /out/rootfs/tmp \
 && install -m 0755 target/release/clockping /out/clockping \
 && chmod 1777 /out/rootfs/tmp

FROM scratch AS release

COPY --from=release-build /out/rootfs/ /
COPY --from=release-build /out/clockping /clockping
ENTRYPOINT ["/clockping"]
