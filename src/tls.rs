use std::sync::Arc;

use anyhow::Context;
use reqwest::{Client, ClientBuilder};
use rustls::{
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{WebPkiSupportedAlgorithms, ring, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};

pub(crate) fn client_builder(insecure: bool) -> anyhow::Result<ClientBuilder> {
    let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    Ok(Client::builder().tls_backend_preconfigured(client_config(insecure, roots)?))
}

fn client_config(insecure: bool, roots: RootCertStore) -> anyhow::Result<ClientConfig> {
    // Neither OS trust nor a process-global provider is used, including in scratch.
    let provider = Arc::new(ring::default_provider());
    let algorithms = provider.signature_verification_algorithms;
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("failed to configure TLS protocol versions")?;
    let config = if insecure {
        // BuiltRustls ignores reqwest's tls_danger_accept_invalid_certs flag.
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipCertificateVerification(algorithms)))
    } else {
        builder.with_root_certificates(roots)
    };
    Ok(config.with_no_client_auth())
}

#[derive(Debug)]
struct SkipCertificateVerification(WebPkiSupportedAlgorithms);

impl ServerCertVerifier for SkipCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.0)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.0)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.supported_schemes()
    }
}

#[cfg(test)]
#[path = "../tests/tls/regression.rs"]
pub(crate) mod tests;
