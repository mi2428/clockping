"""Loopback TLS trust check: python3 tests/tls_smoke.py BINARY SCRATCH_PARENT.

Requires Python 3 and OpenSSL. Fails visibly; does not install roots or contact
external endpoints. All temporary certificate files live under SCRATCH_PARENT.
"""
from contextlib import contextmanager
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import os
from pathlib import Path
import ssl
import subprocess
import sys
import tempfile
import threading


def generate_fixtures(directory):
    """Test-only CA, valid/expired/wrong-name leaves; keys never leave scratch."""
    def openssl(*args):
        result = subprocess.run(["openssl", *args], cwd=directory, capture_output=True, text=True)
        assert result.returncode == 0, (args, result.stderr)

    key_args = ("-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-nodes")
    openssl("req", "-x509", *key_args, "-days", "2", "-subj", "/CN=Clockping Test Root",
            "-addext", "basicConstraints=critical,CA:TRUE", "-addext",
            "keyUsage=critical,keyCertSign,cRLSign", "-keyout", "ca.key", "-out", "ca.pem")
    openssl("req", "-new", *key_args, "-subj", "/CN=localhost",
            "-keyout", "server.key", "-out", "server.csr")
    (directory / "index").write_text("")
    (directory / "serial").write_text("02\n")
    (directory / "ca.cnf").write_text(
        "[ca]\ndefault_ca=local\n[local]\ndatabase=index\nserial=serial\n"
        "new_certs_dir=.\ncertificate=ca.pem\nprivate_key=ca.key\n"
        "default_md=sha256\npolicy=policy\n[policy]\ncommonName=supplied\n")
    for serial, (name, days, san) in enumerate((
        ("valid", "2", "DNS:localhost,IP:127.0.0.1"),
        ("expired", "2", "DNS:localhost,IP:127.0.0.1"),
        ("mismatch", "2", "DNS:wrong.invalid"),
    ), 1):
        (directory / "extensions.cnf").write_text(
            f"basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\n"
            f"extendedKeyUsage=serverAuth\nsubjectAltName={san}\n")
        if name == "expired":
            openssl("ca", "-batch", "-notext", "-config", "ca.cnf", "-in", "server.csr",
                    "-startdate", "20000101000000Z", "-enddate", "20010101000000Z",
                    "-extfile", "extensions.cnf", "-out", "expired.pem")
        else:
            openssl("x509", "-req", "-in", "server.csr", "-CA", "ca.pem", "-CAkey", "ca.key",
                    "-set_serial", str(serial), "-days", days, "-extfile", "extensions.cnf",
                    "-out", f"{name}.pem")


class Handler(BaseHTTPRequestHandler):
    def respond(self):
        self.server.methods.append(self.command)
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.send_response(204)
        self.send_header("Content-Length", "0")
        self.end_headers()

    do_HEAD = respond
    do_PUT = respond
    do_DELETE = respond

    def log_message(self, *args):
        pass


@contextmanager
def endpoint(context=None):
    with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
        server.methods = []
        if context is not None:
            server.socket = context.wrap_socket(server.socket, server_side=True)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            scheme = "https" if context is not None else "http"
            yield f"{scheme}://127.0.0.1:{server.server_port}/", server.methods
        finally:
            server.shutdown()
            thread.join()


def main():
    binary = str(Path(sys.argv[1]).resolve(strict=True))
    scratch = Path(sys.argv[2]).resolve(strict=True)
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("CLOCKPING_") and not key.lower().endswith("_proxy")}

    def probe(url, *args, status=0):
        result = subprocess.run(
            [binary, "--ts.preset", "none", *args, "-c", "1", "-W", "2", url],
            env=env, capture_output=True, text=True, timeout=20)
        assert result.returncode == status, (result.returncode, result.stdout, result.stderr)
        summary = f"1 probes transmitted, {int(status == 0)} replies received"
        assert summary in result.stdout, (result.stdout, result.stderr)
        return result

    with tempfile.TemporaryDirectory(prefix="clockping-tls-", dir=scratch) as directory:
        cert, key = Path(directory) / "cert.pem", Path(directory) / "key.pem"
        subprocess.run(
            ["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
             "-subj", "/CN=localhost", "-addext", "subjectAltName=IP:127.0.0.1",
             "-keyout", str(key), "-out", str(cert)], check=True, capture_output=True)
        context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        context.load_cert_chain(cert, key)
        with endpoint() as (http_url, methods):
            probe(http_url, "http")
            assert methods == ["HEAD"], methods
        with endpoint(context) as (https_url, methods):
            probe(https_url, "http", status=1)
            assert methods == [], methods
            probe(https_url, "http", "--insecure")
            assert methods == ["HEAD"], methods
            result = probe(
                https_url, "--push.url", https_url, "--push.timeout", "2s",
                "--push.delete-on-exit", "http", "--insecure")
            assert "failed to push metrics:" in result.stderr, result.stderr
            assert "failed to delete Pushgateway metrics:" in result.stderr, result.stderr
            assert methods == ["HEAD", "HEAD"], methods
    print("PASS: HTTP, untrusted TLS rejection, --insecure; HTTPS Pushgateway PUT/DELETE remain strict")


if __name__ == "__main__":
    if sys.argv[1] == "--fixtures":
        generate_fixtures(Path(sys.argv[2]).resolve(strict=True))
    else:
        main()
