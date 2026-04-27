#!/usr/bin/env python3
"""GTP Echo target that stops responding while its container stays up."""

from __future__ import annotations

import argparse
import signal
import socket
import sys
import threading
from collections.abc import Callable
from types import FrameType
from typing import Literal, NamedTuple, cast

Mode = Literal["v1", "v2"]
Responder = Callable[[bytes], bytes | None]

BUFFER_SIZE: int = 2048
GTP_VERSION_SHIFT: int = 5
GTP_ECHO_REQUEST: int = 1
GTP_ECHO_RESPONSE: int = 2
GTP_V1_MIN_ECHO_REQUEST_SIZE: int = 12
GTP_V2_MIN_ECHO_REQUEST_SIZE: int = 8
GTP_V1_HAS_SEQUENCE: int = 0x02
GTP_V2_HAS_TEID: int = 0x08

GTP_V1_ECHO_RESPONSE_PREFIX: bytes = bytes(
    [0x32, GTP_ECHO_RESPONSE, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00]
)
GTP_V2_ECHO_RESPONSE_PREFIX: bytes = bytes([0x40, GTP_ECHO_RESPONSE, 0x00, 0x04])


class Config(NamedTuple):
    """Command line configuration."""

    mode: Mode
    port: int
    responses_before_down: int


def parse_args(argv: list[str]) -> Config:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["v1", "v2"], required=True)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--responses-before-down", type=int, required=True)
    args = parser.parse_args(argv)
    return Config(
        mode=cast(Mode, args.mode),
        port=args.port,
        responses_before_down=args.responses_before_down,
    )


def v1_response(packet: bytes) -> bytes | None:
    """Build a GTPv1 Echo Response for a valid Echo Request packet."""
    if len(packet) < GTP_V1_MIN_ECHO_REQUEST_SIZE:
        return None
    if packet[0] >> GTP_VERSION_SHIFT != 1 or packet[1] != GTP_ECHO_REQUEST:
        return None

    sequence = packet[8:10] if packet[0] & GTP_V1_HAS_SEQUENCE else b"\x00\x00"
    return GTP_V1_ECHO_RESPONSE_PREFIX + sequence + b"\x00\x00"


def v2_response(packet: bytes) -> bytes | None:
    """Build a GTPv2 Echo Response for a valid Echo Request packet."""
    if len(packet) < GTP_V2_MIN_ECHO_REQUEST_SIZE:
        return None
    if packet[0] >> GTP_VERSION_SHIFT != 2 or packet[1] != GTP_ECHO_REQUEST:
        return None

    if packet[0] & GTP_V2_HAS_TEID:
        if len(packet) < 12:
            return None
        sequence = packet[8:11]
    else:
        sequence = packet[4:7]
    return GTP_V2_ECHO_RESPONSE_PREFIX + sequence + b"\x00"


def install_signal_handlers(stop: threading.Event) -> None:
    """Install termination handlers so Compose can stop the container cleanly."""

    def request_stop(_signum: int, _frame: FrameType | None) -> None:
        stop.set()

    signal.signal(signal.SIGINT, request_stop)
    signal.signal(signal.SIGTERM, request_stop)


def respond_until_down(config: Config, responder: Responder, stop: threading.Event) -> None:
    """Respond to a fixed number of Echo Requests, then close the UDP socket."""
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.bind(("0.0.0.0", config.port))
        sock.settimeout(0.2)
        print(
            f"transient gtp echo server {config.mode} listening on udp/{config.port}; "
            f"down after {config.responses_before_down} responses",
            flush=True,
        )

        responses = 0
        while responses < config.responses_before_down and not stop.is_set():
            try:
                packet, peer = sock.recvfrom(BUFFER_SIZE)
            except TimeoutError:
                continue
            response = responder(packet)
            if response is None:
                continue
            sock.sendto(response, peer)
            responses += 1
            print(
                f"sent response {responses}/{config.responses_before_down} to {peer}",
                flush=True,
            )

    print(f"transient gtp echo server stopped listening on udp/{config.port}", flush=True)


def wait_until_stopped(stop: threading.Event) -> None:
    """Keep the container alive after the UDP socket goes down."""
    while not stop.wait(1.0):
        pass


def main() -> int:
    """Run the transient GTP Echo responder."""
    config = parse_args(sys.argv[1:])
    if config.responses_before_down < 1:
        raise ValueError("--responses-before-down must be at least 1")

    stop = threading.Event()
    install_signal_handlers(stop)
    responder: Responder = v1_response if config.mode == "v1" else v2_response
    respond_until_down(config, responder, stop)
    wait_until_stopped(stop)
    return 0


if __name__ == "__main__":
    sys.exit(main())
