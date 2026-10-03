"""cargo build && python3 tests/http_proxy_contract.py; loopback only, no forwarding."""

from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import socket
import subprocess


binary = Path(os.environ.get("CLOCKPING_BIN", Path(__file__).resolve().parents[1] / "target/debug/clockping"))
clean_env = {key: value for key, value in os.environ.items()
             if not key.lower().endswith("_proxy") and not key.startswith("CLOCKPING_")
             and key != "REQUEST_METHOD"}


def probe(target, flag, env):
    result = subprocess.run(
        [str(binary), "--out.format", "json", "http", flag, "-c", "1", "-W", "1", target],
        env=env, text=True, capture_output=True, timeout=15,
    )
    event = json.loads(result.stdout.splitlines()[0])
    return result, event


def respond(proxy):
    with proxy.accept()[0] as connection:
        connection.settimeout(10)
        request = b""
        while b"\r\n\r\n" not in request:
            chunk = connection.recv(4096)
            assert chunk, request
            request += chunk
        connection.sendall(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        return request.decode()


for family, host, flag in ((socket.AF_INET, "127.0.0.1", "-4"), (socket.AF_INET6, "::1", "-6")):
    # Bound but not listening: the origin cannot respond, and its port is reserved.
    with socket.socket(family) as origin, socket.socket(family) as proxy:
        origin.bind((host, 0))
        proxy.bind((host, 0))
        proxy.listen()
        proxy.settimeout(10)
        authority = f"localhost:{origin.getsockname()[1]}"
        target = f"http://{authority}/probe"
        proxy_host = f"[{host}]" if family == socket.AF_INET6 else host
        proxy_url = f"http://{proxy_host}:{proxy.getsockname()[1]}"
        env = dict(clean_env, HTTP_PROXY=proxy_url, http_proxy=proxy_url, NO_PROXY="", no_proxy="")
        with ThreadPoolExecutor(max_workers=1) as executor:
            request = executor.submit(respond, proxy)
            result, event = probe(target, flag, env)
            assert result.returncode == 0, result.stdout + result.stderr
            assert request.result(timeout=10).startswith(f"HEAD {target} HTTP/1.1\r\n")
        assert event["status"] == "reply" and event["peer"] == authority, event
        assert ["status", "204"] in event["detail"], event

        # NO_PROXY and cleared environment both restore direct, failing reachability.
        for direct_env, direct_flag in (
            (dict(env, NO_PROXY="localhost", no_proxy="localhost"), flag),
            (clean_env, flag),
            (env, "-6" if flag == "-4" else "-4"),
        ):
            result, event = probe(target, direct_flag, direct_env)
            assert result.returncode != 0 and event["status"] != "reply", event
            proxy.setblocking(False)
            try:
                connection, _ = proxy.accept()
            except BlockingIOError:
                pass
            else:
                connection.close()
                raise AssertionError("unexpected connection to bypassed/mismatched proxy")
print("HTTP proxy contract: IPv4/IPv6 proxy replies, URL peer, bypass and family constraints verified")
