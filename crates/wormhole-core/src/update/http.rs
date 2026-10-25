//! The HTTP client used for the feed and archive downloads.
//!
//! ureq rather than reqwest: we only need blocking GETs from a background thread, and ureq does
//! that without pulling in a tokio runtime (reqwest's blocking client spins one up internally).
//! TLS is rustls with the platform verifier, so certificates are checked by macOS's trust store
//! (including enterprise roots and revocation) and we don't ship a bundled root list.

use std::sync::Arc;
use std::time::Duration;

use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use super::UpdateError;

fn agent(url: &str, body_timeout: Duration, user_agent: &str) -> Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::PlatformVerifier)
        .unversioned_rustls_crypto_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .build();
    Agent::config_builder()
        .tls_config(tls)
        // An HTTPS URL must never be redirected to plain HTTP. Loopback test URLs are plain HTTP
        // to begin with, so the flag can only be set for HTTPS ones.
        .https_only(url.to_ascii_lowercase().starts_with("https://"))
        .max_redirects(5)
        .timeout_connect(Some(Duration::from_secs(15)))
        // ureq only offers a total body budget, not a per-read stall timeout; without one a
        // stalled connection would pin the background thread forever.
        .timeout_recv_body(Some(body_timeout))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .user_agent(user_agent)
        .build()
        .new_agent()
}

/// Feed requests: small bodies, so a short budget.
pub(crate) const FEED_TIMEOUT: Duration = Duration::from_secs(30);
/// Archive downloads: generous enough for ~500 MB over a slow connection.
pub(crate) const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60 * 60);

pub(crate) fn get(
    url: &str,
    body_timeout: Duration,
) -> Result<ureq::http::Response<ureq::Body>, UpdateError> {
    request(url, body_timeout, concat!("Wormhole-Updater/", env!("CARGO_PKG_VERSION")), &[])
}

/// A GET with its own user agent and extra headers: the feed's, which carry the anonymous count.
pub(crate) fn request(
    url: &str,
    body_timeout: Duration,
    user_agent: &str,
    headers: &[(&str, &str)],
) -> Result<ureq::http::Response<ureq::Body>, UpdateError> {
    let mut request = agent(url, body_timeout, user_agent).get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    request.call().map_err(|e| UpdateError::Network(format!("GET {url}: {e}")))
}
