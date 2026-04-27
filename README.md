# clockping

Timestamped generic pinger CLI.

## Examples

```sh
clockping icmp 8.8.8.8
clockping icmp -c 5 -i 0.2 -W 1 8.8.8.8
clockping icmp --pinger=/usr/bin/ping -w 1 8.8.8.8

clockping tcp example.com:443
clockping http https://example.com/
clockping http -X GET --ok-status 200,204,300-399 https://example.com/health
clockping gtp v1u 192.0.2.10
clockping gtp v1c 192.0.2.10
clockping gtp v2c 192.0.2.10
```

Custom timestamps use a strftime-like format:

```sh
clockping --timestamp-format "%Y-%m-%d %H:%M:%S%.3f %z" icmp 8.8.8.8
clockping --timestamp rfc3339 --json tcp example.com:443
```

## Shell completion

Completion scripts for bash, zsh, and fish are tracked in `completions/`.
Install them with the release binary by passing `COMPLETION=1`:

```sh
make install COMPLETION=1
```

You can also print a script directly:

```sh
clockping completion bash
clockping completion zsh
clockping completion fish
```

## Version metadata

`clockping --version` includes the package version, git describe, commit,
commit date, build date, build profile, target, and host. `clockping -V` keeps
the short package-version output for scripts that only need the semantic
version.

## ICMP modes

`clockping icmp` uses native ICMP by default. It currently accepts the common
ping-compatible options `-4`, `-6`, `-c`, `-i`, `-W`, `-w`, `-s`, `-t`, `-I`,
`-n`, `-q`, `-D`, and `-O`.

Use `--pinger` when OS-specific ping behavior or unsupported options are needed:

```sh
clockping icmp --pinger=/usr/bin/ping [PING_ARGS...]
```

In wrapper mode, `clockping` prefixes each output line from the external command
with its own timestamp.

For native ICMP compatibility flags, `-n` uses the resolved numeric address in
clockping's target label, `-D` keeps clockping timestamps enabled even when the
global timestamp preset is `none`, and `-O` annotates timeout events as
outstanding replies.

## HTTP mode

`clockping http` sends a `HEAD` request by default and measures the time until
response headers are received. Use `-X GET` to send `GET` instead. Redirects are
not followed unless `-L`/`--location` is set.

Responses with status codes in `--ok-status` count as replies. The default is
`200-399`; pass comma-separated values and ranges such as `200,204,300-399` to
override it. Additional request headers can be supplied with repeated
`-H 'Name: value'` options. HTTPS uses Rustls with embedded webpki roots, so the
scratch release image does not need an OS CA bundle.

## Tests

```sh
make check
make integration
```

The Docker Compose e2e test starts TCP and GTP targets on a private Docker
network, then runs the ignored Rust integration test in `tests/integration_test.rs`
against those targets. It covers
native ICMP, external `--pinger`, TCP, HTTP, GTPv1-U, GTPv1-C, GTPv2-C, and JSON
timestamp formatting.
