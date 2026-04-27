#!/usr/bin/env python3
"""Control endpoint that can drop the container network during ICMP tests."""

from __future__ import annotations

import argparse
import signal
import socket
import subprocess
import threading
from types import FrameType

READ_SIZE: int = 1024


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="0.0.0.0")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--interface", default="eth0")
    return parser.parse_args()


def install_signal_handlers(stop: threading.Event) -> None:
    """Install termination handlers so Compose can stop the container cleanly."""

    def request_stop(_signum: int, _frame: FrameType | None) -> None:
        stop.set()

    signal.signal(signal.SIGINT, request_stop)
    signal.signal(signal.SIGTERM, request_stop)


def listen_for_down_command(host: str, port: int, interface: str, stop: threading.Event) -> None:
    """Listen for a control connection and bring the network interface down."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind((host, port))
        listener.listen()
        listener.settimeout(0.2)
        print(f"icmp down control listening on {host}:{port}", flush=True)

        while not stop.is_set():
            try:
                connection, peer = listener.accept()
            except TimeoutError:
                continue
            except OSError:
                if stop.is_set():
                    return
                raise

            with connection:
                payload = connection.recv(READ_SIZE).strip()
            if payload != b"down":
                print(f"ignored control connection from {peer}", flush=True)
                continue

            print(f"bringing {interface} down after control request from {peer}", flush=True)
            subprocess.run(["ip", "link", "set", "dev", interface, "down"], check=True)
            return


def wait_until_stopped(stop: threading.Event) -> None:
    """Keep the container alive after its network interface goes down."""
    while not stop.wait(1.0):
        pass


def main() -> None:
    """Run the ICMP down control endpoint."""
    args = parse_args()
    stop = threading.Event()
    install_signal_handlers(stop)
    listen_for_down_command(args.host, args.port, args.interface, stop)
    wait_until_stopped(stop)


if __name__ == "__main__":
    main()
