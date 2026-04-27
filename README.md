# clockping

A multi-protocol, multi-target pinger for watching hosts go dark.

## Installation

### macOS (Homebrew)

Prebuilt macOS binaries are available from the Homebrew tap:

```console
$ brew tap mi2428/clockping
$ brew install clockping
```

### Build from source

```console
$ make install               # install the binary only
$ make install COMPLETION=1  # install the binary and shell completions
```

## CLI Usage

Add `--push.*` or `--metrics.*` options when you need live interval metrics.

```console
$ clockping

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
      --timestamp <TIMESTAMP>
          Timestamp preset for human-readable output [default: local] [possible values: local, rfc3339, unix, unix-ms, none]
      --timestamp-format <TIMESTAMP_FORMAT>
          strftime-like timestamp format, similar to `date +"..."`
      --json
          Emit JSON Lines instead of text
  -C, --colored
          Colorize human-readable output with ANSI escape sequences
  -h, --help
          Print help
  -V, --version
          Print version

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

Completion scripts for bash, zsh, and fish are tracked in `completions/`.
Install them with the release binary by passing `COMPLETION=1`:

```console
$ make install COMPLETION=1
$ clockping completion bash  # Print a script directly
$ clockping completion zsh
$ clockping completion fish
```

Use `--colored` to add ANSI colors to human-readable output. JSON Lines output stays uncolored.
Custom timestamps use a strftime-like format:

```console
$ clockping --timestamp-format "%Y-%m-%d %H:%M:%S%.3f %z" icmp 8.8.8.8
$ clockping --timestamp rfc3339 --json tcp example.com:443
$ clockping --colored --timestamp none icmp 1.1.1.1
```

`clockping` can write one metrics event per probe and can push Prometheus text
exposition to a Pushgateway:

```sh
clockping --metrics.file clockping.jsonl tcp -c 5 example.com:443
clockping --metrics.file clockping.prom --metrics.format prometheus \
  --metrics.prefix nettest --metrics.label site=tokyo tcp example.com:443
clockping --push.url 127.0.0.1:9091 --push.label scenario=sample \
  tcp example.com:443
```

Each option also has an environment default: `CLOCKPING_PUSH_URL`,
`CLOCKPING_PUSH_DELETE_ON_EXIT`, `CLOCKPING_PUSH_INTERVAL`,
`CLOCKPING_PUSH_JOB`, `CLOCKPING_PUSH_LABELS`, `CLOCKPING_PUSH_RETRIES`,
`CLOCKPING_PUSH_TIMEOUT`, `CLOCKPING_PUSH_USER_AGENT`,
`CLOCKPING_METRICS_FILE`, `CLOCKPING_METRICS_FORMAT`,
`CLOCKPING_METRICS_LABELS`, and `CLOCKPING_METRICS_PREFIX`.
For migration parity with `iperf3-rs`, the matching `IPERF3_*` names are also
accepted as fallback aliases when the `CLOCKPING_*` variable is not set.

### ICMP mode

For native ICMP compatibility flags, `-n` uses the resolved numeric address in clockping's target label, `-D` keeps clockping timestamps enabled even when the global timestamp preset is `none`, and `-O` annotates timeout events as outstanding replies.

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
  -C, --colored                           Colorize human-readable output with ANSI escape sequences
  -h, --help                              Print help
```

External `--pinger` mode remains a transparent wrapper around one external ping-compatible command line.
Use it when OS-specific ping behavior or unsupported options are needed:

```sh
clockping icmp --pinger=/usr/bin/ping [PING_ARGS...]
```

In wrapper mode, `clockping` prefixes each output line from the external command with its own timestamp.

### TCP mode

`clockping tcp` requires `host:port` targets. Bare hosts are rejected so the probed service is explicit. Use `-4` or `-6` to restrict DNS results to IPv4 or IPv6.

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
  -C, --colored              Colorize human-readable output with ANSI escape sequences
  -i, --interval <INTERVAL>  Seconds between probes. Fractions are accepted, e.g. 0.2 [default: 1]
  -W, --timeout <TIMEOUT>    Per-probe connect timeout in seconds [default: 1]
  -w, --deadline <DEADLINE>  Stop the command after this many seconds
  -q, --quiet                Suppress per-probe output and only print the summary
  -h, --help                 Print help

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

`clockping http` sends a `HEAD` request by default and measures the time until response headers are received.
Use `-X GET` to send `GET` instead.
Redirects are not followed unless `-L`/`--location` is set.
Use `-4` or `-6` to restrict DNS results to IPv4 or IPv6.

Responses with status codes in `--ok-status` count as replies.
The default is `200-399`; pass comma-separated values and ranges such as `200,204,300-399` to override it.
Additional request headers can be supplied with repeated `-H 'Name: value'` options.
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
  -C, --colored                Colorize human-readable output with ANSI escape sequences
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
  -C, --colored  Colorize human-readable output with ANSI escape sequences
  -h, --help     Print help

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

### Tests

```sh
make check
make e2e
```

The Docker Compose e2e test starts TCP and GTP targets on a private Docker network, then runs the ignored Rust E2E test in `tests/e2e_test.rs` against those targets.
It covers native ICMP, external `--pinger`, TCP, HTTP, GTPv1-U, GTPv1-C, GTPv2-C, and JSON timestamp formatting.

### Release

Release builds are driven from a local machine to avoid spending GitHub Actions compute on release artifact and image builds:

```sh
make release TAG=v1.0.0
```

The release target builds `dist/` binaries and checksums, pushes the multi-arch scratch image to GHCR, then creates or updates the GitHub Release with the local artifacts.
It expects `gh` to be authenticated and Docker Buildx to be able to push `linux/amd64,linux/arm64` images.

## License

MIT License. See [LICENSE](LICENSE) for details.