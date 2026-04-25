# clockping

Timestamped generic pinger CLI.

## Examples

```sh
clockping icmp 8.8.8.8
clockping icmp -c 5 -i 0.2 -W 1 8.8.8.8
clockping icmp --pinger=/usr/bin/ping -w 1 8.8.8.8

clockping tcp example.com:443
clockping gtp v1u 192.0.2.10
clockping gtp v1c 192.0.2.10
clockping gtp v2c 192.0.2.10
```

Custom timestamps use a strftime-like format:

```sh
clockping --timestamp-format "%Y-%m-%d %H:%M:%S%.3f %z" icmp 8.8.8.8
clockping --timestamp rfc3339 --json tcp example.com:443
```

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

## Tests

```sh
cargo test
cargo clippy --all-targets -- -D warnings
docker compose -f docker-compose.test.yml up --build --abort-on-container-exit --exit-code-from sut
```

The Docker Compose e2e test starts TCP and GTP targets on a private Docker
network, then runs the built `clockping` binary against those targets. It covers
native ICMP, external `--pinger`, TCP, GTPv1-U, GTPv1-C, GTPv2-C, and JSON
timestamp formatting.
