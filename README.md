# clockping

A multi-protocol, multi-target pinger for watching hosts go dark.

[![](https://github.com/mi2428/clockping/blob/main/screencast.gif?raw=true)](https://github.com/mi2428/clockping/blob/main/screencast.gif)

## Installation

### macOS (Homebrew)

Install the prebuilt macOS binary from the Homebrew tap.

```console
$ brew tap mi2428/clockping
$ brew install clockping
```

### Build from source

Install Rust and Cargo first, then build and install the binary with `make install`.
By default, the binary is installed to `~/.local/bin/clockping`.
Set `INSTALL_BINDIR` if you want to install it somewhere else.

```console
$ git clone https://github.com/mi2428/clockping
$ make -C clockping install
```

>[!TIP]
> Prebuilt binaries are also available from GitHub Releases for macOS and Linux, with amd64 and arm64 builds for each platform.
> Pick the asset that matches your machine, make it executable, and place it on your `PATH`.
>
> ```console
> $ curl -L -o clockping https://github.com/mi2428/clockping/releases/download/v0.9.0/clockping-v0.9.0-darwin-arm64
> $ chmod +x ./clockping
> ```

## Usage

Pick a probe mode, pass one or more targets, and let clockping print timestamped probe events until the count, deadline, or interrupt stops the run.
Output and metrics options are global, so they can be placed before or after the mode name.

```console
$ clockping --help

A multi-protocol, multi-target pinger for watching hosts go dark

Usage: clockping [OPTIONS] <COMMAND>

Commands:
  icmp        ICMP echo ping. Native by default; use --pinger to wrap system ping
  tcp         TCP connect ping
  http        HTTP request ping. HEAD by default; use -X GET to send GET
  gtp         GTP Echo ping
  completion  Generate a shell completion script
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Output Options:
      --ts.preset <PRESET>   Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>   strftime-like timestamp format, similar to `date +"..."`
      --out.format <FORMAT>  Output format [default: text] [possible values: text, json]
      --out.colored          Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>             Push interval metrics to a Pushgateway URL
      --push.delete-on-exit        Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>   Aggregate interval samples before pushing window metrics
      --push.job <JOB>             Pushgateway job name
      --push.label <KEY=VALUE>     Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>           Retry failed Pushgateway requests N times
      --push.timeout <DURATION>    Pushgateway request timeout
      --push.user-agent <VALUE>    HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>        Write live interval metrics to a file
      --metrics.format <FORMAT>    Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>  Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>    Prometheus metric name prefix
```

Shell completions for bash, zsh, and fish are tracked in `completions/`.
Print a script directly with the `completion` subcommand.

```console
$ clockping completion bash  # Print a script directly
$ clockping completion zsh
$ clockping completion fish
```

Human-readable output is timestamped by default.
Use presets, a custom strftime-like format, JSON Lines, or ANSI colors depending on whether the output is for a terminal, log file, or parser.

```console
$ clockping --ts.format "%Y-%m-%d %H:%M:%S%.3f %z" icmp 8.8.8.8
$ clockping --ts.preset rfc3339 --out.format json tcp example.com:443
$ clockping --out.colored --ts.preset none icmp 1.1.1.1
```

Metrics can be written to a local file, pushed to a Prometheus Pushgateway, or both.
Supported file formats are listed below.

- `jsonl`: keeps every probe event as JSON Lines.
- `prometheus`: keeps the latest Prometheus text snapshot.

The included `docker-compose.yml` provides a local visualization stack.
Run `docker compose up` to start Pushgateway, Prometheus, and Grafana.
It exposes Pushgateway on `:9091`, Prometheus on `:9090`, and Grafana on `:3000`.

```text
$ clockping --metrics.file clockping.jsonl tcp -c 5 example.com:443
$ clockping --metrics.file clockping.prom --metrics.format prometheus \
    --metrics.prefix nettest --metrics.label site=tokyo tcp example.com:443
$ clockping --push.url 127.0.0.1:9091 --push.label scenario=sample \
    tcp example.com:443
```

CLI values override environment defaults.
The supported `CLOCKPING_*` defaults are listed below.

| Environment variable | Option | Meaning |
| --- | --- | --- |
| `CLOCKPING_PUSH_URL` | `--push.url` | Pushgateway endpoint URL. |
| `CLOCKPING_PUSH_DELETE_ON_EXIT` | `--push.delete-on-exit` | Delete the Pushgateway grouping key after the run exits. |
| `CLOCKPING_PUSH_INTERVAL` | `--push.interval` | Aggregate samples locally before pushing window metrics. |
| `CLOCKPING_PUSH_JOB` | `--push.job` | Pushgateway job name. |
| `CLOCKPING_PUSH_LABELS` | `--push.label` | Comma-separated Pushgateway grouping labels. |
| `CLOCKPING_PUSH_RETRIES` | `--push.retries` | Retry count for failed Pushgateway requests. |
| `CLOCKPING_PUSH_TIMEOUT` | `--push.timeout` | Per-request Pushgateway timeout. |
| `CLOCKPING_PUSH_USER_AGENT` | `--push.user-agent` | Pushgateway HTTP User-Agent. |
| `CLOCKPING_METRICS_FILE` | `--metrics.file` | Metrics output file path. |
| `CLOCKPING_METRICS_FORMAT` | `--metrics.format` | Metrics file format, either `jsonl` or `prometheus`. |
| `CLOCKPING_METRICS_LABELS` | `--metrics.label` | Comma-separated Prometheus file labels. |
| `CLOCKPING_METRICS_PREFIX` | `--metrics.prefix` | Prometheus metric name prefix. |

### ICMP mode

Native ICMP is the default and supports the common ping-style options shown below.
Notable compatibility flags are `-n` for numeric target labels, `-D`/`--timestamp` to force clockping timestamps even when `--ts.preset none` is set, and `-O` to mark timeout events as outstanding replies.

```console
$ clockping icmp --help

ICMP echo ping. Native by default; use --pinger to wrap system ping

Usage: clockping icmp [OPTIONS] <DESTINATION>...
       clockping icmp --pinger <PROGRAM> [PING_ARGS]...

Arguments:
  <DESTINATION>...  Destination host or IP address. Repeat for multiple targets
  [PING_ARGS]...    With --pinger, arguments passed unchanged to the external command

Options:
  -4                                      Use IPv4 only
  -6                                      Use IPv6 only
  -c, --count <COUNT>                     Stop after count probes. Default is to run until interrupted
  -i, --interval <SECONDS>                Seconds between probes. Fractions are accepted, e.g. 0.2 [default: 1]
  -W, --timeout <SECONDS>                 Per-probe timeout in seconds [default: 1]
  -w, --deadline <SECONDS>                Stop the command after this many seconds
  -s, --size <BYTES>                      Number of payload bytes [default: 56]
  -t, --ttl <TTL>                         IP TTL / hop limit
  -I, --interface-or-source <INTERFACE_OR_SOURCE>
                                          Interface name or source address
  -n, --numeric                           Numeric output only. Accepted for ping compatibility
  -q, --quiet                             Suppress per-probe output and only print the summary
  -D, --timestamp                         Accepted for ping compatibility. clockping timestamps every event by default
  -O, --report-outstanding                Report outstanding reply before sending next packet
      --pinger <PROGRAM>                  Run an external ping-compatible command instead of native ICMP
  -h, --help                              Print help
  -V, --version                           Print version

Output Options:
      --ts.preset <PRESET>              Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>              strftime-like timestamp format, similar to `date +"..."`
      --out.format <FORMAT>             Output format [default: text] [possible values: text, json]
      --out.colored                     Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>                    Push interval metrics to a Pushgateway URL
      --push.delete-on-exit               Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>          Aggregate interval samples before pushing window metrics
      --push.job <JOB>                    Pushgateway job name
      --push.label <KEY=VALUE>            Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>                  Retry failed Pushgateway requests N times
      --push.timeout <DURATION>           Pushgateway request timeout
      --push.user-agent <VALUE>           HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>               Write live interval metrics to a file
      --metrics.format <FORMAT>           Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>         Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>           Prometheus metric name prefix
```

Use external `--pinger` mode when you need OS-specific ping behavior or options that native mode does not support.

```text
$ clockping icmp --pinger=/usr/bin/ping [PING_ARGS...]
```

Wrapper mode passes arguments through unchanged and prefixes each external output line with a clockping timestamp.

### TCP mode

TCP mode measures connect latency to explicit `host:port` targets.
Bare hosts are rejected so the service being probed is unambiguous.
Use `-4` or `-6` to restrict DNS results.

```console
$ clockping tcp --help

TCP connect ping

Usage: clockping tcp [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  Targets as host:port. Repeat for multiple targets

Options:
  -4                         Use IPv4 only
  -6                         Use IPv6 only
  -c, --count <COUNT>        Stop after count probes. Default is to run until interrupted
  -i, --interval <INTERVAL>  Seconds between probes. Fractions are accepted, e.g. 0.2 [default: 1]
  -W, --timeout <TIMEOUT>    Per-probe connect timeout in seconds [default: 1]
  -w, --deadline <DEADLINE>  Stop the command after this many seconds
  -q, --quiet                Suppress per-probe output and only print the summary
  -h, --help                 Print help
  -V, --version              Print version

Output Options:
      --ts.preset <PRESET>   Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>   strftime-like timestamp format, similar to `date +"..."`
      --out.format <FORMAT>  Output format [default: text] [possible values: text, json]
      --out.colored          Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>             Push interval metrics to a Pushgateway URL
      --push.delete-on-exit        Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>   Aggregate interval samples before pushing window metrics
      --push.job <JOB>             Pushgateway job name
      --push.label <KEY=VALUE>     Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>           Retry failed Pushgateway requests N times
      --push.timeout <DURATION>    Pushgateway request timeout
      --push.user-agent <VALUE>    HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>        Write live interval metrics to a file
      --metrics.format <FORMAT>    Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>  Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>    Prometheus metric name prefix
```

### HTTP mode

HTTP mode sends `HEAD` by default and measures time to response headers.
Use `-X GET` when the endpoint requires a body-capable request.
Status codes in `--ok-status` count as replies, redirects require `-L`, repeated `-H 'Name: value'` options add headers, and `-4`/`-6` restrict DNS results.
HTTPS uses Rustls with embedded webpki roots, so the scratch release image does not need an OS CA bundle.

```console
$ clockping http --help

HTTP request ping. HEAD by default; use -X GET to send GET

Usage: clockping http [OPTIONS] <TARGET>...

Arguments:
  <TARGET>...  Target URLs. If no scheme is given, http:// is assumed

Options:
  -4                           Use IPv4 only
  -6                           Use IPv6 only
  -c, --count <COUNT>          Stop after count probes. Default is to run until interrupted
  -i, --interval <INTERVAL>    Seconds between probes. Fractions are accepted, e.g. 0.2 [default: 1]
  -W, --timeout <TIMEOUT>      Per-probe request timeout in seconds [default: 1]
  -w, --deadline <DEADLINE>    Stop the command after this many seconds
  -X, --method <METHOD>        HTTP method to send [default: head] [possible values: head, get]
      --ok-status <OK_STATUS>  Treat these HTTP status codes as successful, e.g. 200,204,300-399 [default: 200-399]
  -H, --header <HEADERS>       Add a request header. Repeat for multiple headers
  -L, --location               Follow HTTP redirects
  -k, --insecure               Skip TLS certificate verification
  -q, --quiet                  Suppress per-probe output and only print the summary
  -h, --help                   Print help
  -V, --version                Print version

Output Options:
      --ts.preset <PRESET>   Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>   strftime-like timestamp format, similar to `date +"..."`
      --out.format <FORMAT>  Output format [default: text] [possible values: text, json]
      --out.colored          Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>             Push interval metrics to a Pushgateway URL
      --push.delete-on-exit        Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>   Aggregate interval samples before pushing window metrics
      --push.job <JOB>             Pushgateway job name
      --push.label <KEY=VALUE>     Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>           Retry failed Pushgateway requests N times
      --push.timeout <DURATION>    Pushgateway request timeout
      --push.user-agent <VALUE>    HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>        Write live interval metrics to a file
      --metrics.format <FORMAT>    Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>  Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>    Prometheus metric name prefix
```

### GTP mode

GTP mode sends Echo Requests for GTPv1-U, GTPv1-C, or GTPv2-C.
The subcommands share count, interval, timeout, deadline, quiet, and optional UDP port controls.

```console
$ clockping gtp --help

GTP Echo ping

Usage: clockping gtp [OPTIONS] <COMMAND>

Commands:
  v1u   GTPv1-U Echo Request, default UDP/2152
  v1c   GTPv1-C Echo Request, default UDP/2123
  v2c   GTPv2-C Echo Request, default UDP/2123
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Output Options:
      --ts.preset <PRESET>   Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --ts.format <FORMAT>   strftime-like timestamp format, similar to `date +"..."`
      --out.format <FORMAT>  Output format [default: text] [possible values: text, json]
      --out.colored          Colorize human-readable output with ANSI escape sequences

Metrics Options:
      --push.url <URL>             Push interval metrics to a Pushgateway URL
      --push.delete-on-exit        Delete this Pushgateway grouping key after the run exits
      --push.interval <DURATION>   Aggregate interval samples before pushing window metrics
      --push.job <JOB>             Pushgateway job name
      --push.label <KEY=VALUE>     Add a Pushgateway grouping label. Repeat for multiple labels
      --push.retries <N>           Retry failed Pushgateway requests N times
      --push.timeout <DURATION>    Pushgateway request timeout
      --push.user-agent <VALUE>    HTTP User-Agent for Pushgateway requests
      --metrics.file <PATH>        Write live interval metrics to a file
      --metrics.format <FORMAT>    Metrics file format: jsonl or prometheus
      --metrics.label <KEY=VALUE>  Add a Prometheus file sample label. Repeat for multiple labels
      --metrics.prefix <PREFIX>    Prometheus metric name prefix
```

## Development

```console
$ make

Development
  build                      Build the host binary into bin/
  install                    Build and install the host binary into INSTALL_BINDIR
  fmt                        Format Rust sources. Use CHECK_ONLY=1 to check without writing
  lint                       Run clippy with warnings treated as errors
  doc                        Build rustdoc with warnings treated as errors
  test                       Run unit tests
  check                      Run formatting, lint, rustdoc, and tests
  clean                      Remove local build artifacts

Demo
  vhs                        Record the README live CUI demo GIF with VHS

Distribution
  release                    Build dist, publish a GitHub release, and update Homebrew. Requires TAG=vX.Y.Z
  dist                       Build release binaries into dist/. Use OS=darwin,linux and ARCH=amd64,arm64
  dist-smoke                 Smoke-test Linux dist binaries in a Debian container
  checksums                  Write SHA-256 checksums for dist artifacts

Help
  help                       Show this help message

Variables:
  TAG                        Release tag for make release, for example v0.1.0
  GIT_REMOTE                 Release git remote, defaults to origin
  HOMEBREW_TAP               Set to 0 to skip Homebrew tap updates, defaults to 1
  HOMEBREW_TAP_DIR           Homebrew tap checkout, defaults to ../homebrew-clockping
  HOMEBREW_TAP_REMOTE        Homebrew tap git remote, defaults to origin
  HOMEBREW_TAP_SLUG          brew tap slug, defaults to GitHub owner/clockping
  HOMEBREW_TAP_README_TITLE  Homebrew tap README title, defaults to homebrew-clockping
  HOMEBREW_DESC              Homebrew formula description
  HOMEBREW_FORMULA_CLASS     Homebrew Ruby class, defaults to Clockping
  OS                         Release OS list for make dist, defaults to darwin,linux
  ARCH                       Release arch list for make dist, defaults to amd64,arm64
  INSTALL_BINDIR             Install directory, defaults to /Users/teo/.local/bin
  VHS                        VHS command for make vhs, defaults to vhs
  VHS_DEMO_COMMAND           Demo command for make vhs
  VHS_DEMO_DELAY_SCALE       Demo scan delay scale for make vhs, defaults to 1

Examples:
  make fmt CHECK_ONLY=1                       # Check formatting without writing
  make check                                  # Run local quality gates
  make vhs                                    # Record screencast.gif from deterministic demo data
  make dist OS=darwin,linux ARCH=amd64,arm64  # Build release binaries and checksums
  make release TAG=v0.1.0                     # Publish a GitHub release and update Homebrew
```

### Tests

`make check` runs the local quality gate.
The Docker Compose E2E test network can be run directly when real TCP, HTTP, ICMP, and GTP targets are needed, including target-down cases that must keep producing timestamped events.

```console
$ make check
$ docker compose -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from sut
```

### Release

Release workflows are kept as `.yml.disabled` files, so releases are driven from a local machine instead of GitHub Actions.

```console
$ make release TAG=v1.0.0
```

The release target builds tag-named `dist/` binaries and checksums, creates or updates the GitHub Release, uploads the artifacts, and publishes the Homebrew formula.
It expects `gh` to be authenticated, Docker to be available for Linux release builds, and [`../homebrew-clockping`](https://github.com/mi2428/homebrew-clockping) to be a clean local checkout of the tap repo.
Set `HOMEBREW_TAP=0` to skip the Homebrew tap update.

## License

MIT License.
See [LICENSE](LICENSE) for details.
