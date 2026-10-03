# Dependency maintenance assessment

Evidence checked on 2026-10-03 with Rust 1.95.0. A lockfile advisory match
does not by itself establish an enabled vulnerable execution path.

## Compatible refresh (#24)

The manifest constraints already permit these releases; only resolution changes
are needed. Reqwest remains 0.12.28 and surge-ping remains 0.8.4.

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
