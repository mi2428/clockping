# Dependency maintenance assessment

Evidence checked on 2026-10-03 with Rust 1.95.0. A lockfile advisory match
does not by itself establish an enabled vulnerable execution path.

## Compatible refresh (#24)

The manifest constraints already permit these releases; only resolution changes
were needed. During #24, reqwest stayed at 0.12.28 (subsequently migrated below)
and surge-ping remained 0.8.4.

| Dependency | Before | Selected | Declared MSRV |
| --- | --- | --- | --- |
| anyhow | 1.0.102 | 1.0.104 | 1.68 |
| async-trait | 0.1.89 | 0.1.92 | 1.71 |
| chrono | 0.4.44 | 0.4.45 | 1.62 |
| clap | 4.6.1 | 4.6.7 | 1.85 |
| clap_complete | 4.6.2 | 4.6.11 | 1.85 |
| libc | 0.2.186 | 0.2.190 | 1.65 |
| serde | 1.0.228 | 1.0.229 | 1.56 |
| serde_json | 1.0.149 | 1.0.151 | 1.71 |
| socket2 | 0.6.3 | 0.6.5 | 1.70 |
| thiserror | 2.0.18 | 2.0.21 | 1.77 |
| tokio | 1.52.1 | 1.53.1 | 1.71 |

Version/MSRV evidence: the version records at
`https://crates.io/api/v1/crates/<crate>`; the pinned toolchain also built the
enabled dependency graph. Necessary derive dependencies introduce syn 3.0.6
alongside syn 2; no application feature is added.

### Advisory disposition

- [RUSTSEC-2026-0190](https://rustsec.org/advisories/RUSTSEC-2026-0190.html):
  anyhow is now above the patched 1.0.103 threshold. No application caller of
  `Error::downcast_mut` was found before the update.
- [RUSTSEC-2026-0185](https://rustsec.org/advisories/RUSTSEC-2026-0185.html):
  quinn-proto 0.11.14 -> 0.11.15, the first patched release. This avoids the
  additional dependency changes of 0.11.19. HTTP/3 remains disabled;
  `cargo tree --locked -e features -i quinn-proto --target all` has no active
  path. CVE-2026-25800 and GHSA-4w2j-m93h-cj5j are aliases, not extra defects.
- Rustls remains 0.23.45 / rustls-webpki 0.103.15. Feature trees retain ring
  and embedded webpki-roots 1.0.7, not native TLS or OS-root loading.
- A read-only OSV `querybatch` scan of all 197 registry lock entries returned
  no advisory matches on the evidence date. This is not a cargo-audit run,
  a proof of absence of vulnerabilities, or a runtime reachability scan.

### Verification and integration gate

- `make test`: 82 unit and 21 integration tests passed; Docker E2E explicitly
  ignored. An initial build exceeded the runner's 120-second timeout; rerunning
  with a longer limit completed successfully.
- A later parallel run and an isolated `sigint_interrupts_active_probe` run
  failed with SIGINT before any output; that startup race was also recorded
  before this refresh. `RUST_TEST_THREADS=1 make test` passed all 103 tests,
  and `cargo test --locked protocol::` passed 26 focused tests. The observed
  intermittent failure is not represented as an unconditional test-gate pass.
- `make doc`, `cargo check --locked`, and
  `cargo check --locked --target x86_64-apple-darwin` passed on the macOS host.
- Native loopback smoke probes with `icmp -4 ... 127.0.0.1` and
  `icmp -6 ... ::1` each received a reply without elevation.
- SHA256 comparisons found identical top-level/protocol/completion help and
  zsh/PowerShell/Elvish completions. Bash now omits the six literal positional
  placeholders (`<TARGET>...` / `[ARGS]...`) from word suggestions; fish changes
  indentation only. No CLI options disappeared.
- Bash/fish generated artifacts now match the updated generator byte-for-byte;
  zsh is unchanged and also matches. The six deleted Bash words are usage
  placeholders, not commands/options; the filename fallback is retained.
  `bash -n` (also macOS `/bin/bash`), `fish --no-config -n`, `zsh -n`, and the
  existing completion/help integration tests passed. Nineteen Bash hook contexts
  on each Bash version retained every real candidate.
- Fish changes indentation only, with ten before/after hook outputs identical.
  **Existing runtime limitation:** Fish 4.9.3 rejects the generated global
  `argparse` spec `ts.preset=` in both versions, blocking positive command
  completion coverage. No workaround was added; generator synchronization does
  not fix this separate dotted-option parsing defect.
- The combined development branch passed `make check`: 105 unit tests and
  35 integration tests, with native Pushgateway and Docker E2E separately ignored.
  Native formatting/lint and SIGINT readiness fixes were integrated; the earlier
  branch-local failures above are retained as evidence, not the final gate result.
  Generators are rechecked against the combined CLI, including the long-help
  hints introduced by the label-precedence documentation.
- Linux, Compose, and scratch runtime checks remain unexecuted because the
  Docker daemon is unavailable; no daemon, interface, or toolchain was changed.

## Reqwest 0.13 migration (#25)

**Implemented: reqwest 0.12.28 -> 0.13.5**, the latest stable non-yanked release
in the 2026-10-03 crates.io API response (MSRV 1.85; pinned Rust 1.95).
This supersedes the earlier deferral; no 0.12 fallback remains.

Sources: [0.13.5 changelog](https://github.com/seanmonstar/reqwest/blob/v0.13.5/CHANGELOG.md),
the published Cargo features, and
[ClientBuilder source](https://docs.rs/reqwest/0.13.5/src/reqwest/async_impl/client.rs.html).

### TLS and dependency boundaries

- `default-features = false` remains; only `rustls-no-provider` is enabled.
  No native TLS, aws-lc, system-proxy, HTTP/2, HTTP/3, compression, JSON, query,
  or form feature is added. Surge-ping is unchanged by this migration.
- Both production clients use `src/tls.rs`: embedded webpki-roots 1.0.7 trust
  anchors and a client-local ring provider, not a global provider installation
  or OS roots. Rustls 0.23.45 / rustls-webpki 0.103.15 / ring 0.17.14 are retained
  and Rustls/webpki-roots are now explicit direct dependencies.
- Reqwest's public `rustls-no-provider` feature pulls in platform-verifier
  0.7.1 and its platform dependencies. They remain in the dependency graph,
  but neither client uses that verifier: `tls_backend_preconfigured` supplies
  the complete Rustls configuration. No CA bundle is copied or loaded.
- `BuiltRustls` bypasses builder-level certificate settings. HTTP `-k` therefore
  selects an explicit verifier that bypasses chain, hostname and expiry checks
  but delegates TLS 1.2/1.3 handshake signatures to Rustls. Pushgateway always
  selects strict WebPKI verification, including for PUT and DELETE.
- Preconfigured TLS uses type downcasting and is semver-sensitive: future
  reqwest/Rustls changes must retain matching Rustls types and rerun these tests.
  A mismatch fails client construction, not a silent backend fallback.
- Environment proxy routing, custom DNS/address-family selection, methods,
  headers, redirects, timeout/retry, output and metrics paths are unchanged.
  `no_proxy()` appears only in isolated test clients, not production builders.

### Runnable regression coverage

`make check` includes the Rust loopback TLS regressions in
`tests/tls/regression.rs`. They require Python 3 and OpenSSL, generate disposable
CA/key files under `target/`, and remove them on completion. Local-root injection
is test-only; no production trust setting or CLI/schema option was added.

| Contract | Coverage |
| --- | --- |
| Strict HTTPS / HTTP `-k` | Trusted local-root success, separate unknown-issuer/expired/hostname rejection with zero HTTP requests, explicit certificate bypass success. |
| TLS signatures | TLS 1.2 and 1.3 succeed with valid signatures and reject deliberately corrupted server signatures even under `-k`. |
| HTTPS Pushgateway | Strict unknown-root PUT/DELETE rejection and trusted PUT/DELETE success for TLS 1.2/1.3; assert grouping path, user-agent, content-type and body. CLI smoke checks HTTP `-k` never relaxes Pushgateway trust. |
| HTTP contracts | HEAD/GET, custom headers/status, default no-follow/limit 10, localhost IPv4/IPv6 custom DNS, stalled-header timeout, header-only RTT with withheld body; existing metrics integration tests retain retries/cancel/deadline/file/output checks. |
| Proxies | Standalone loopback HTTP/HTTPS CONNECT checks: IPv4/IPv6, URL peer, family constraints, HTTPS_PROXY/ALL_PROXY and NO_PROXY, strict rejection and explicit `-k`. |
| Linux/scratch | Integration gate still required: release-image public HTTPS and HTTPS Pushgateway without host roots. Native macOS tests are not a Linux/scratch runtime pass. |

```sh
make check
cargo build --locked
python3 tests/tls_smoke.py target/debug/clockping target
python3 tests/http_proxy_contract.py
cargo test --locked protocol::http::
cargo test --locked --test integration_test integration::metrics::
```

The Python smokes are explicit checks, not automatically included in `make test`.
Missing tools, timeouts, unexpected responses or leaked insecure PUT/DELETE fail
visibly. The worker does not run containers; Linux/scratch checks are the
supervisor's post-integration responsibility.

Native macOS verification passed: `make check` (112 unit + 35 integration tests;
native Pushgateway and Docker E2E explicitly ignored), 11 focused HTTP tests,
15 focused metrics integration tests, both Python smokes, and
`cargo check --locked --target x86_64-apple-darwin`. A read-only strict HTTPS
HEAD to `https://example.com/` also returned 200 using the embedded public roots.
No external PUT/DELETE was sent. These results do not claim Linux/scratch or
a new advisory scan of the changed lockfile.

## Surge-ping 0.9 assessment (#26)

**Decision: defer migration; retain 0.8.4.** The assessment and baseline coverage
are complete, not approval of 0.9 on Linux/macOS. Integrate native maintenance
from #10 first. A source-level packet-semantics risk, plus missing Linux runtime
coverage, makes a version-only upgrade inappropriate.

Sources: [0.9.0 release](https://github.com/kolapapa/surge-ping/releases/tag/0.9.0),
[0.9.1 changelog](https://github.com/kolapapa/surge-ping/blob/0.9.1/CHANGELOG.md),
`cargo info surge-ping@0.9.1 --verbose`, and the published 0.8.4/0.9.1
`config.rs`, `client.rs`, `ping.rs`, and ICMP decoders.

### API, platform, and behavior inventory

- 0.9.0 raises MSRV to 1.85 / edition 2024 and moves socket2 0.5 -> 0.6,
  thiserror 1 -> 2, rand 0.9 -> 0.10, and pnet_packet 0.34 -> 0.35.
  Our toolchain already supports its MSRV, and our direct socket2 is 0.6.
  Clockping does not consume the breaking public `AsyncSocket::get_type()`
  socket2 type. A future migration needs the manifest/lock update; no change to
  the currently called API signatures was found. This is not a 0.9 build claim.
- `ConfigBuilder::{kind, ttl, bind, interface, interface_index}`, `Client::new`,
  `Client::pinger`, `Pinger::{scope_id, timeout, ping}`, packet getters, and
  `SurgeError::Timeout` remain. Source binding still uses SocketAddr; macOS
  binding uses the nonzero index and IPv4/IPv6 socket options; Linux interface
  binding uses SO_BINDTODEVICE. IPv6 scope_id is still applied on send.
- DGRAM-first/RAW-second socket selection remains. Linux permission diagnostics
  change; approved runners must already permit ICMP and interface binding.
  Do not change ping_group_range, grant capabilities, or elevate inside tests.
- 0.9.1 fixes Client-clone lifetime, duplicate waiter replacement, cancellation
  cleanup, and ICMP error decoding/routing. These have maintenance value, but
  the application owns one Client per prober, does not clone it, serializes
  ping calls per prober, and retains `_client` while its Pinger is alive.
  Linux DGRAM sockets can use kernel-owned identifiers rather than the hint;
  do not mistake that for a broken per-prober identifier allocator.
- **Reject a version-only migration:** 0.9.1 `recv_task` routes intermediate
  ICMP errors by `packet.real_destination()` (0.8.4 used sender address), and
  `Pinger::ping` returns that packet as Ok. Clockping's current `probe` maps
  every Ok V4/V6 packet to `ProbeOutcome::Reply`, without checking EchoReply.
  Thus newly delivered time-exceeded/unreachable packets can become false
  successful replies, affecting loss/up/exit status. This is source-level
  evidence, not a reproduced live-router failure. A reviewed migration must
  classify packet type at the shared native probe boundary and test it before
  adoption; do not conceal the change with a fallback to the old dependency.

### Focused coverage and approval checklist

`tests/icmp_smoke.py` is a standalone native-only check requiring no extra
Python dependency. It uses only lo0/lo and loopback addresses, changes no
interface, and raises on permission failure, missing IPv6, or unmet assertions.

```sh
cargo build --locked
python3 tests/icmp_smoke.py target/debug/clockping
cargo test --locked protocol::icmp::
```

| Contract | Evidence / required approval coverage |
| --- | --- |
| Successful interface selection | macOS lo0, IPv4 and IPv6: two replies each. Run the same mandatory check on Linux lo with preconfigured privileges. |
| Invalid interface selection | Both families fail explicitly with unknown-interface error; existing unit test also passes. |
| Source-address selection | 127.0.0.1 and ::1 binding each receive two replies; add opposite-family/nonlocal source rejection with native validation work. |
| IPv4/IPv6 | Unbound loopback succeeds for both; IPv6 interface check exercises scope_id assignment, not link-local routing. Add controlled link-local scope and hostname family filtering. |
| Independent probers | Existing identifier uniqueness unit test passes; two simultaneous same-target probers, sequence 0/1, each receive both replies for both families. Does not inspect on-wire kernel identifiers. |
| ICMP error packet semantics | Add deterministic IPv4/IPv6 time-exceeded/unreachable and quoted-header fixtures; assert they are not EchoReply/up/success. Required before adopting the new routing. |
| Socket and lifecycle behavior | Verify DGRAM and RAW permissions, timeout and cancellation on each supported platform; baseline smoke does not force RAW. |
| Linux/Compose | Existing Compose exercises native IPv4, external ping, and target-down; add mandatory interface/IPv6/source coverage. Docker unavailable, so no Linux/Compose pass is claimed. |

The macOS smoke and eight focused ICMP tests passed with 0.8.4. Run these
explicit checks on the selected 0.9 candidate, alongside `make check` and
Compose native scenarios, before approval. Upstream's own platform-skipping
tests are not a substitute for required project coverage. There is no OSV match
for the current lock graph; version age alone is not a new security defect.
