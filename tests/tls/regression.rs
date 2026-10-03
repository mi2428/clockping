//! Loopback regression fixtures, shared by the TLS, HTTP and Pushgateway tests.
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rustls::{
    ServerConfig, ServerConnection, SignatureAlgorithm, StreamOwned, SupportedProtocolVersion,
    pki_types::{PrivateKeyDer, pem::PemObject},
    sign::{CertifiedKey, Signer, SigningKey, SingleCertAndKey},
};

use super::*;

pub(crate) struct Fixture {
    directory: PathBuf,
    roots: RootCertStore,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "target/tls-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut fixture = Self {
            directory,
            roots: RootCertStore::empty(),
        };
        let output = Command::new("python3")
            .args([
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/tls_smoke.py"),
                "--fixtures",
            ])
            .arg(&fixture.directory)
            .output()
            .expect("TLS regression requires Python 3 and OpenSSL");
        assert!(output.status.success(), "{output:?}");
        fixture
            .roots
            .add(CertificateDer::from_pem_file(fixture.directory.join("ca.pem")).unwrap())
            .unwrap();
        fixture
    }

    pub(crate) fn builder(&self, insecure: bool) -> ClientBuilder {
        Client::builder()
            .tls_backend_preconfigured(client_config(insecure, self.roots.clone()).unwrap())
            .no_proxy()
            .timeout(Duration::from_secs(2))
    }

    pub(crate) fn server(
        &self,
        certificate: &str,
        version: &'static SupportedProtocolVersion,
        bad_signature: bool,
        attempts: usize,
    ) -> Server {
        let provider = Arc::new(ring::default_provider());
        let cert = CertificateDer::from_pem_file(self.directory.join(format!("{certificate}.pem")))
            .unwrap();
        let key = PrivateKeyDer::from_pem_file(self.directory.join("server.key")).unwrap();
        let mut key = CertifiedKey::from_der(vec![cert], key, &provider).unwrap();
        if bad_signature {
            key.key = Arc::new(BadSigningKey(key.key));
        }
        let config = Arc::new(
            ServerConfig::builder_with_provider(provider)
                .with_protocol_versions(&[version])
                .unwrap()
                .with_no_client_auth()
                .with_cert_resolver(Arc::new(SingleCertAndKey::from(key))),
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let thread = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..attempts {
                let deadline = Instant::now() + Duration::from_secs(5);
                let socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "TLS fixture accept timed out");
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("TLS fixture accept: {error}"),
                    }
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut stream =
                    StreamOwned::new(ServerConnection::new(config.clone()).unwrap(), socket);
                let mut request = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break, // Rejected handshakes must send no HTTP request.
                        Ok(length) => request.extend_from_slice(&buffer[..length]),
                    }
                    let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .map(|value| value.parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if request.len() >= end + 4 + length {
                        assert_eq!(stream.conn.protocol_version(), Some(version.version));
                        stream.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                        stream.flush().unwrap();
                        requests.push(String::from_utf8(request).unwrap());
                        break;
                    }
                }
            }
            requests
        });
        Server { address, thread }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

pub(crate) struct Server {
    pub(crate) address: SocketAddr,
    thread: JoinHandle<Vec<String>>,
}

impl Server {
    pub(crate) fn url(&self) -> String {
        format!("https://{}/", self.address)
    }

    pub(crate) fn requests(self) -> Vec<String> {
        self.thread.join().unwrap()
    }
}

#[derive(Debug)]
struct BadSigningKey(Arc<dyn SigningKey>);

impl SigningKey for BadSigningKey {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn Signer>> {
        self.0
            .choose_scheme(offered)
            .map(|signer| Box::new(BadSigner(signer)) as Box<dyn Signer>)
    }

    fn algorithm(&self) -> SignatureAlgorithm {
        self.0.algorithm()
    }
}

#[derive(Debug)]
struct BadSigner(Box<dyn Signer>);

impl Signer for BadSigner {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, rustls::Error> {
        let mut signature = self.0.sign(message)?;
        *signature.last_mut().unwrap() ^= 1;
        Ok(signature)
    }

    fn scheme(&self) -> SignatureScheme {
        self.0.scheme()
    }
}

#[tokio::test]
async fn insecure_still_verifies_tls12_and_tls13_handshake_signatures() {
    let fixture = Fixture::new();
    for version in [&rustls::version::TLS12, &rustls::version::TLS13] {
        for bad_signature in [false, true] {
            let server = fixture.server("valid", version, bad_signature, 1);
            let result = client_builder(true)
                .unwrap()
                .no_proxy()
                .timeout(Duration::from_secs(2))
                .build()
                .unwrap()
                .head(server.url())
                .send()
                .await;
            if bad_signature {
                let error = result.unwrap_err();
                assert!(format!("{error:?}").contains("BadSignature"), "{error:?}");
                assert!(server.requests().is_empty());
            } else {
                assert_eq!(result.unwrap().status(), reqwest::StatusCode::NO_CONTENT);
                assert_eq!(server.requests().len(), 1);
            }
        }
    }
}

#[tokio::test]
async fn strict_trust_rejects_unknown_expired_and_wrong_hostname() {
    let fixture = Fixture::new();
    for (certificate, trust_root, reason) in [
        ("valid", false, "UnknownIssuer"),
        ("expired", true, "Expired"),
        ("mismatch", true, "NotValidForName"),
    ] {
        let server = fixture.server(certificate, &rustls::version::TLS13, false, 1);
        let builder = if trust_root {
            fixture.builder(false)
        } else {
            client_builder(false)
                .unwrap()
                .no_proxy()
                .timeout(Duration::from_secs(2))
        };
        let error = builder
            .build()
            .unwrap()
            .head(server.url())
            .send()
            .await
            .unwrap_err();
        assert!(format!("{error:?}").contains(reason), "{error:?}");
        assert!(server.requests().is_empty());

        let server = fixture.server(certificate, &rustls::version::TLS13, false, 1);
        let response = client_builder(true)
            .unwrap()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
            .head(server.url())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
        assert_eq!(server.requests().len(), 1);
    }
}
