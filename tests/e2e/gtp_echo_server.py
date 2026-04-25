#!/usr/bin/env python3
import argparse
import socket
import sys


def v1_response(packet: bytes) -> bytes | None:
    if len(packet) < 12:
        return None
    if packet[0] >> 5 != 1 or packet[1] != 1:
        return None

    sequence = packet[8:10] if packet[0] & 0x02 else b"\x00\x00"
    return b"\x32\x02\x00\x04" + b"\x00\x00\x00\x00" + sequence + b"\x00\x00"


def v2_response(packet: bytes) -> bytes | None:
    if len(packet) < 8:
        return None
    if packet[0] >> 5 != 2 or packet[1] != 1:
        return None

    if packet[0] & 0x08:
        if len(packet) < 12:
            return None
        sequence = packet[8:11]
    else:
        sequence = packet[4:7]
    return b"\x40\x02\x00\x04" + sequence + b"\x00"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["v1", "v2"], required=True)
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()

    responder = v1_response if args.mode == "v1" else v2_response
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(("0.0.0.0", args.port))
    print(f"gtp echo server {args.mode} listening on udp/{args.port}", flush=True)

    while True:
        packet, peer = sock.recvfrom(2048)
        response = responder(packet)
        if response is None:
            continue
        sock.sendto(response, peer)


if __name__ == "__main__":
    sys.exit(main())
