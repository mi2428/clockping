#!/usr/bin/env python3
"""HTTP target that intentionally stops listening while its container stays up."""

from __future__ import annotations

import argparse
import signal
import socket
import threading
from types import FrameType

READ_SIZE: int = 4096
HTTP_RESPONSE: bytes = (
    b"HTTP/1.1 200 OK\r\n"
    b"Content-Length: 2\r\n"
    b"Connection: close\r\n"
    b"\r\n"
    b"ok"
)


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--accepts-before-down", type=int, required=True)
    return parser.parse_args()


def install_signal_handlers(stop: threading.Event) -> None:
    """Install termination handlers so Compose can stop the container cleanly."""

    def request_stop(_signum: int, _frame: FrameType | None) -> None:
        stop.set()

    signal.signal(signal.SIGINT, request_stop)
    signal.signal(signal.SIGTERM, request_stop)


def serve_until_down(host: str, port: int, accepts_before_down: int, stop: threading.Event) -> None:
    """Serve a fixed number of HTTP connections, then close the listener."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind((host, port))
        listener.listen()
        listener.settimeout(0.2)
        print(
            f"transient http server listening on {host}:{port}; "
            f"down after {accepts_before_down} accepts",
            flush=True,
        )

        accepted = 0
        while accepted < accepts_before_down and not stop.is_set():
            try:
                connection, peer = listener.accept()
            except TimeoutError:
                continue
            except OSError:
                if stop.is_set():
                    return
                raise

            accepted += 1
            print(f"accepted connection {accepted}/{accepts_before_down} from {peer}", flush=True)
            with connection:
                connection.settimeout(0.5)
                try:
                    request = connection.recv(READ_SIZE)
                except TimeoutError:
                    request = b""
                if request:
                    connection.sendall(HTTP_RESPONSE)

    print(f"transient http server stopped listening on {host}:{port}", flush=True)


def wait_until_stopped(stop: threading.Event) -> None:
    """Keep the container alive after the listener goes down."""
    while not stop.wait(1.0):
        pass


def main() -> None:
    """Run the transient HTTP server."""
    args = parse_args()
    if args.accepts_before_down < 1:
        raise ValueError("--accepts-before-down must be at least 1")

    stop = threading.Event()
    install_signal_handlers(stop)
    serve_until_down(args.host, args.port, args.accepts_before_down, stop)
    wait_until_stopped(stop)


if __name__ == "__main__":
    main()
