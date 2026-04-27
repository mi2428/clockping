#!/usr/bin/env python3
"""Minimal GTP Echo responder for clockping Docker e2e tests."""

from __future__ import annotations

import argparse
import socket
import sys
from collections.abc import Callable
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


def parse_args(argv: list[str]) -> Config:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["v1", "v2"], required=True)
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args(argv)
    return Config(mode=cast(Mode, args.mode), port=args.port)


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


def main() -> int:
    """Run the UDP responder until the process is terminated."""
    config = parse_args(sys.argv[1:])
    responder: Responder = v1_response if config.mode == "v1" else v2_response

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", config.port))
    print(f"gtp echo server {config.mode} listening on udp/{config.port}", flush=True)

    while True:
        packet, peer = sock.recvfrom(BUFFER_SIZE)
        response = responder(packet)
        if response is None:
            continue
        sock.sendto(response, peer)


if __name__ == "__main__":
    sys.exit(main())
