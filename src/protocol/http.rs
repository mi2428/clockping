use std::{net::Ipv6Addr, ops::RangeInclusive, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{
    Client, Method, StatusCode, Url, Version,
    header::{CONTENT_LENGTH, HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
};
use tokio::time::Instant;

use crate::{event::ProbeOutcome, runner::Prober};

pub struct HttpProberConfig {
    pub target: String,
    pub method: Method,
    pub timeout: Duration,
    pub headers: Vec<(String, String)>,
    pub follow_redirects: bool,
    pub insecure: bool,
    pub ok_statuses: Vec<RangeInclusive<u16>>,
}

pub struct HttpProber {
    target: String,
    url: Url,
    method: Method,
    client: Client,
    headers: HeaderMap,
    ok_statuses: Vec<RangeInclusive<u16>>,
}

impl HttpProber {
    pub fn new(config: HttpProberConfig) -> anyhow::Result<Self> {
        let url = normalize_url(&config.target)?;
        let mut builder = Client::builder()
            .use_rustls_tls()
            .timeout(config.timeout)
            .redirect(if config.follow_redirects {
                Policy::limited(10)
            } else {
                Policy::none()
            });
        if config.insecure {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build().context("failed to build HTTP client")?;
        let headers = build_headers(config.headers)?;
        let target = url.to_string();

        Ok(Self {
            target,
            url,
            method: config.method,
            client,
            headers,
            ok_statuses: config.ok_statuses,
        })
    }

    fn is_ok_status(&self, status: StatusCode) -> bool {
        let code = status.as_u16();
        self.ok_statuses.iter().any(|range| range.contains(&code))
    }
}

#[async_trait]
impl Prober for HttpProber {
    fn protocol(&self) -> &'static str {
        "http"
    }

    fn target(&self) -> &str {
        &self.target
    }

    async fn probe(&mut self, _seq: u64) -> ProbeOutcome {
        let started = Instant::now();
        let request = self
            .client
            .request(self.method.clone(), self.url.clone())
            .headers(self.headers.clone());

        match request.send().await {
            Ok(response) => {
                // `send` completes once response headers are available. Do not
                // consume the body here; GET remains explicit but still measures
                // header RTT rather than download throughput.
                let rtt = started.elapsed();
                let status = response.status();
                if !self.is_ok_status(status) {
                    return ProbeOutcome::Error(format!("unexpected HTTP status {status}"));
                }

                let peer = response_peer(response.url());
                let mut detail = vec![
                    ("method".to_string(), self.method.as_str().to_string()),
                    ("status".to_string(), status.as_u16().to_string()),
                    ("version".to_string(), version_text(response.version())),
                ];
                if response.url() != &self.url {
                    detail.push(("url".to_string(), response.url().to_string()));
                }
                if let Some(content_length) = content_length(response.headers()) {
                    detail.push(("content_length".to_string(), content_length.to_string()));
                }

                ProbeOutcome::Reply {
                    rtt,
                    peer,
                    bytes: None,
                    ttl: None,
                    detail,
                }
            }
            Err(error) if error.is_timeout() => ProbeOutcome::Timeout {
                detail: vec![("method".to_string(), self.method.as_str().to_string())],
            },
            Err(error) => ProbeOutcome::Error(error.to_string()),
        }
    }
}

fn normalize_url(target: &str) -> anyhow::Result<Url> {
    let candidate = if target.contains("://") {
        target.to_string()
    } else {
        format!("http://{target}")
    };
    let url = Url::parse(&candidate).with_context(|| format!("invalid URL: {target}"))?;
    anyhow::ensure!(
        matches!(url.scheme(), "http" | "https"),
        "unsupported URL scheme: {}",
        url.scheme()
    );
    Ok(url)
}

fn build_headers(headers: Vec<(String, String)>) -> anyhow::Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in headers {
        let name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid HTTP header name: {name:?}"))?;
        let value = HeaderValue::from_str(&value)
            .with_context(|| format!("invalid value for HTTP header {name}"))?;
        map.append(name, value);
    }
    Ok(map)
}

fn response_peer(url: &Url) -> String {
    let Some(host) = url.host_str() else {
        return "unknown".to_string();
    };
    let host = match host.parse::<Ipv6Addr>() {
        Ok(_) => format!("[{host}]"),
        Err(_) => host.to_string(),
    };
    match url.port_or_known_default() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}

fn version_text(version: Version) -> String {
    format!("{version:?}")
}

fn content_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use std::{ops::RangeInclusive, time::Duration};

    use reqwest::Method;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    #[tokio::test]
    async fn http_probe_replies_to_local_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let len = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..len]);
            assert!(request.starts_with("HEAD / HTTP/1.1"));
            stream
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });

        let mut prober = HttpProber::new(test_config(addr.to_string())).unwrap();
        let outcome = prober.probe(0).await;
        server.await.unwrap();

        match outcome {
            ProbeOutcome::Reply { peer, detail, .. } => {
                assert_eq!(peer, addr.to_string());
                assert!(detail.contains(&("method".to_string(), "HEAD".to_string())));
                assert!(detail.contains(&("status".to_string(), "204".to_string())));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_probe_sends_get_and_custom_header() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let len = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..len]);
            assert!(request.starts_with("GET /health HTTP/1.1"));
            assert!(request.contains("\r\nx-clockping-test: yes\r\n"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .await
                .unwrap();
        });

        let mut config = test_config(format!("http://{addr}/health"));
        config.method = Method::GET;
        config.headers = vec![("x-clockping-test".to_string(), "yes".to_string())];
        let mut prober = HttpProber::new(config).unwrap();
        let outcome = prober.probe(0).await;
        server.await.unwrap();

        match outcome {
            ProbeOutcome::Reply { detail, .. } => {
                assert!(detail.contains(&("method".to_string(), "GET".to_string())));
                assert!(detail.contains(&("content_length".to_string(), "2".to_string())));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_probe_follows_redirect_when_enabled() {
        let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_addr = target_listener.local_addr().unwrap();
        let redirect_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let redirect_addr = redirect_listener.local_addr().unwrap();

        let target_server = tokio::spawn(async move {
            let (mut stream, _) = target_listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            stream
                .write_all(
                    b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let redirect_server = tokio::spawn(async move {
            let (mut stream, _) = redirect_listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{target_addr}/done\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let mut config = test_config(format!("http://{redirect_addr}/"));
        config.follow_redirects = true;
        let mut prober = HttpProber::new(config).unwrap();
        let outcome = prober.probe(0).await;
        redirect_server.await.unwrap();
        target_server.await.unwrap();

        match outcome {
            ProbeOutcome::Reply { detail, .. } => {
                assert!(detail.contains(&("status".to_string(), "204".to_string())));
                assert!(
                    detail.iter().any(|(key, value)| key == "url"
                        && value == &format!("http://{target_addr}/done"))
                );
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[tokio::test]
    async fn http_probe_accepts_configured_status() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });

        let mut config = test_config(addr.to_string());
        config.ok_statuses = vec![RangeInclusive::new(503, 503)];
        let mut prober = HttpProber::new(config).unwrap();
        let outcome = prober.probe(0).await;
        server.await.unwrap();

        assert!(matches!(outcome, ProbeOutcome::Reply { .. }));
    }

    #[tokio::test]
    async fn http_probe_errors_on_unexpected_status() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 1024];
            let _ = stream.read(&mut buf).await.unwrap();
            stream
                .write_all(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });

        let mut prober = HttpProber::new(test_config(addr.to_string())).unwrap();
        let outcome = prober.probe(0).await;
        server.await.unwrap();

        assert!(matches!(
            outcome,
            ProbeOutcome::Error(error) if error.contains("unexpected HTTP status 500")
        ));
    }

    #[test]
    fn normalize_url_defaults_to_http() {
        let url = normalize_url("example.com/path").unwrap();
        assert_eq!(url.as_str(), "http://example.com/path");
    }

    fn test_config(target: String) -> HttpProberConfig {
        HttpProberConfig {
            target,
            method: Method::HEAD,
            timeout: Duration::from_secs(1),
            headers: Vec::new(),
            follow_redirects: false,
            insecure: false,
            ok_statuses: vec![RangeInclusive::new(200, 399)],
        }
    }
}
