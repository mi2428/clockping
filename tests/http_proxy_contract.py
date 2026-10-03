"""cargo build && python3 tests/http_proxy_contract.py; loopback only, no forwarding."""

from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import socket
import ssl
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
from tls_smoke import generate_fixtures


binary = Path(os.environ.get("CLOCKPING_BIN", Path(__file__).resolve().parents[1] / "target/debug/clockping"))
clean_env = {key: value for key, value in os.environ.items()
             if not key.lower().endswith("_proxy") and not key.startswith("CLOCKPING_")
             and key != "REQUEST_METHOD"}


def probe(target, flag, env, *args):
    result = subprocess.run(
        [str(binary), "--out.format", "json", "http", flag, *args, "-c", "1", "-W", "1", target],
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


def respond_connect(proxy, context, authority, insecure):
    with proxy.accept()[0] as connection:
        connection.settimeout(5)
        request = b""
        while b"\r\n\r\n" not in request:
            chunk = connection.recv(4096)
            assert chunk, request
            request += chunk
        assert request.startswith(f"CONNECT {authority} HTTP/1.1\r\n".encode()), request
        connection.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        try:
            with context.wrap_socket(connection, server_side=True) as tls:
                assert insecure, "strict client accepted an unknown CA"
                request = tls.recv(4096)
                assert request.startswith(b"HEAD /probe HTTP/1.1\r\n"), request
                tls.sendall(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        except ssl.SSLError:
            assert not insecure, "insecure client failed the TLS handshake"


with tempfile.TemporaryDirectory(prefix="proxy-tls-", dir=binary.parent.parent) as directory:
    directory = Path(directory)
    generate_fixtures(directory)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(directory / "valid.pem", directory / "server.key")
    for family, host, flag in ((socket.AF_INET, "127.0.0.1", "-4"), (socket.AF_INET6, "::1", "-6")):
        with socket.socket(family) as origin, socket.socket(family) as proxy:
            origin.bind((host, 0))
            proxy.bind((host, 0))
            proxy.listen()
            proxy.settimeout(5)
            authority = f"localhost:{origin.getsockname()[1]}"
            target = f"https://{authority}/probe"
            proxy_host = f"[{host}]" if family == socket.AF_INET6 else host
            proxy_url = f"http://{proxy_host}:{proxy.getsockname()[1]}"
            # Both HTTPS_PROXY and ALL_PROXY/lowercase retain the existing routing contract.
            for variable in ("HTTPS_PROXY", "all_proxy"):
                env = dict(clean_env, **{variable: proxy_url}, NO_PROXY="", no_proxy="")
                for insecure in (False, True):
                    with ThreadPoolExecutor(max_workers=1) as executor:
                        response = executor.submit(respond_connect, proxy, context, authority, insecure)
                        result, event = probe(target, flag, env, *(["-k"] if insecure else []))
                        response.result(timeout=10)
                    assert (result.returncode == 0) == insecure, result.stdout + result.stderr
                    assert (event["status"] == "reply") == insecure, event
                result, event = probe(target, flag, dict(env, NO_PROXY="localhost", no_proxy="localhost"), "-k")
                assert result.returncode != 0 and event["status"] != "reply", event
                proxy.setblocking(False)
                try:
                    connection, _ = proxy.accept()
                except BlockingIOError:
                    pass
                else:
                    connection.close()
                    raise AssertionError("unexpected HTTPS connection with NO_PROXY")
                proxy.settimeout(5)
print("HTTPS proxy contract: IPv4/IPv6 CONNECT, strict trust, -k, HTTPS_PROXY/ALL_PROXY and NO_PROXY verified")
