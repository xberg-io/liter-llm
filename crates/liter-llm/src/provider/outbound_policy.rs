//! Outbound HTTP request policy guard.
//!
//! Allows callers (the proxy server in particular) to constrain which upstream
//! URLs can be reached.  The library default is [`OutboundPolicy::Off`] —
//! preserves backward-compatibility for FFI consumers and embedded applications
//! where the application owner registers their own providers and is trusted.
//! The proxy switches the policy to [`OutboundPolicy::DenyPrivate`] at startup
//! so multi-tenant deployments cannot SSRF into cloud metadata services or
//! private networks.

use std::net::IpAddr;
use std::sync::{OnceLock, RwLock};

use url::Url;

use crate::error::LiterLlmError;

// ── Policy enum ───────────────────────────────────────────────────────────────

/// Controls which upstream URLs the library is allowed to connect to.
///
/// Set once at application startup via [`set_outbound_policy`].  Checked at
/// provider registration time and, when a custom DNS resolver is wired into
/// `reqwest`, at every TCP connection attempt (defense in depth against DNS
/// rebinding).
#[derive(Debug, Clone, Default)]
#[cfg_attr(alef, alef(skip))]
pub enum OutboundPolicy {
    /// No restrictions — library default.  Use only when the application is
    /// the sole registrar of provider URLs and trusts itself.
    #[default]
    Off,

    /// Reject URLs whose host resolves to any private / loopback / link-local
    /// / multicast / CGNAT address.  Recommended for multi-tenant proxies.
    DenyPrivate,

    /// Only allow URLs whose origin (scheme + host + port) matches one of the
    /// provided entries.
    Allowlist(Vec<Url>),
}

// ── Global policy ─────────────────────────────────────────────────────────────

static GLOBAL_POLICY: OnceLock<RwLock<OutboundPolicy>> = OnceLock::new();

fn policy_lock() -> &'static RwLock<OutboundPolicy> {
    GLOBAL_POLICY.get_or_init(|| RwLock::new(OutboundPolicy::default()))
}

/// Set the global outbound policy.
///
/// Subsequent calls to [`validate_outbound_url`] and the per-connection DNS
/// resolver use this policy.  Intended to be called once at application
/// startup before any provider is registered.
#[cfg_attr(alef, alef(skip))]
pub fn set_outbound_policy(policy: OutboundPolicy) {
    *policy_lock().write().expect("outbound policy lock poisoned") = policy;
}

/// Read a snapshot of the current outbound policy.
#[cfg_attr(alef, alef(skip))]
pub fn current_policy() -> OutboundPolicy {
    policy_lock().read().expect("outbound policy lock poisoned").clone()
}

// ── URL validation ────────────────────────────────────────────────────────────

/// Validate `raw_url` against the current outbound policy.
///
/// Under [`OutboundPolicy::DenyPrivate`] the host is resolved via DNS; if
/// *any* returned address is forbidden the call returns
/// [`LiterLlmError::OutboundForbidden`].  This defeats DNS rebinding at
/// registration time.
///
/// Under [`OutboundPolicy::Off`] the function is a no-op and always returns
/// `Ok(())`.
#[cfg_attr(alef, alef(skip))]
pub async fn validate_outbound_url(raw_url: &str) -> Result<(), LiterLlmError> {
    let policy = current_policy();
    if matches!(policy, OutboundPolicy::Off) {
        return Ok(());
    }

    let url = Url::parse(raw_url).map_err(|e| LiterLlmError::OutboundForbidden {
        url: raw_url.to_string(),
        reason: format!("invalid URL: {e}"),
    })?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_string(),
                reason: format!("scheme '{other}' is not allowed; only http/https"),
            });
        }
    }

    match policy {
        OutboundPolicy::Off => Ok(()),
        OutboundPolicy::DenyPrivate => check_deny_private(&url, raw_url).await,
        OutboundPolicy::Allowlist(allowed) => check_allowlist(&url, raw_url, &allowed),
    }
}

#[cfg(target_arch = "wasm32")]
async fn check_deny_private(_url: &Url, _raw: &str) -> Result<(), LiterLlmError> {
    // WASM has no DNS resolver — the sync literal-IP check above is the only
    // SSRF guard available.  Hostname resolution defense in depth lives in the
    // GuardedResolver path which is native-only.
    Ok(())
}

/// Synchronous URL validation — parse + scheme check + literal-IP private range
/// check only.  Does not perform DNS resolution.
///
/// Used from synchronous registration paths.  Catches the obvious
/// `http://169.254.169.254/` literal-IP case without requiring an async
/// context.  DNS-based checks still happen at connect time via
/// [`GuardedResolver`].
#[cfg_attr(alef, alef(skip))]
pub fn validate_outbound_url_sync(raw_url: &str) -> Result<(), LiterLlmError> {
    let policy = current_policy();
    if matches!(policy, OutboundPolicy::Off) {
        return Ok(());
    }

    let url = Url::parse(raw_url).map_err(|e| LiterLlmError::OutboundForbidden {
        url: raw_url.to_string(),
        reason: format!("invalid URL: {e}"),
    })?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_string(),
                reason: format!("scheme '{other}' is not allowed; only http/https"),
            });
        }
    }

    // If the host is already a literal IP address, check it right now.
    // url::Url::host() returns the parsed Host enum; use it to avoid
    // bracket-stripping issues with IPv6 (`host_str()` includes the `[...]`
    // wrapper so `host_str().parse::<IpAddr>()` would fail for IPv6).
    match url.host() {
        Some(url::Host::Ipv4(v4)) if is_forbidden(IpAddr::V4(v4)) => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_string(),
                reason: format!("host is a forbidden address {v4}"),
            });
        }
        Some(url::Host::Ipv6(v6)) if is_forbidden(IpAddr::V6(v6)) => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_string(),
                reason: format!("host is a forbidden address {v6}"),
            });
        }
        _ => {
            // Domain name (or allowed IP) — no sync DNS; GuardedResolver
            // handles hostname-based SSRF at connect time.
        }
    }

    // Allowlist check is purely structural — no DNS needed.
    if let OutboundPolicy::Allowlist(allowed) = policy {
        return check_allowlist(&url, raw_url, &allowed);
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn check_deny_private(url: &Url, raw: &str) -> Result<(), LiterLlmError> {
    let host = url.host_str().ok_or_else(|| LiterLlmError::OutboundForbidden {
        url: raw.to_string(),
        reason: "URL has no host".into(),
    })?;

    let port = url.port_or_known_default().unwrap_or(0);

    let addrs =
        tokio::net::lookup_host(format!("{host}:{port}"))
            .await
            .map_err(|e| LiterLlmError::OutboundForbidden {
                url: raw.to_string(),
                reason: format!("DNS resolution failed: {e}"),
            })?;

    for sa in addrs {
        if is_forbidden(sa.ip()) {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw.to_string(),
                reason: format!("host resolves to forbidden address {}", sa.ip()),
            });
        }
    }
    Ok(())
}

fn check_allowlist(url: &Url, raw: &str, allowed: &[Url]) -> Result<(), LiterLlmError> {
    let origin_match = allowed.iter().any(|a| {
        a.scheme() == url.scheme()
            && a.host_str() == url.host_str()
            && a.port_or_known_default() == url.port_or_known_default()
    });
    if origin_match {
        Ok(())
    } else {
        Err(LiterLlmError::OutboundForbidden {
            url: raw.to_string(),
            reason: "URL not in outbound allowlist".into(),
        })
    }
}

// ── IP classification ─────────────────────────────────────────────────────────

/// Returns `true` if `ip` is in a range that must not be reached from a
/// multi-tenant proxy.
///
/// Covers loopback, unspecified, private (RFC 1918), link-local, multicast,
/// broadcast, CGNAT (100.64/10), IPv4-mapped IPv6, ULA (fc00::/7), and IPv6
/// link-local (fe80::/10).
#[cfg_attr(alef, alef(skip))]
pub fn is_forbidden(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || is_cgnat(v4)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || is_unique_local_v6(v6)
                || is_link_local_v6(v6)
                || v6
                    .to_ipv4_mapped()
                    .map(|m| is_forbidden(IpAddr::V4(m)))
                    .unwrap_or(false)
        }
    }
}

fn is_cgnat(ip: std::net::Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    a == 100 && (64..=127).contains(&b)
}

fn is_unique_local_v6(ip: std::net::Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_link_local_v6(ip: std::net::Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

// ── GuardedResolver ───────────────────────────────────────────────────────────

/// A `reqwest` DNS resolver that filters resolved addresses through the
/// current [`OutboundPolicy`].
///
/// Install via `reqwest::Client::builder().dns_resolver(Arc::new(GuardedResolver))`.
/// Only active when the policy is not [`OutboundPolicy::Off`].  When the
/// policy is `Off` the resolver skips filtering entirely, falling back to
/// standard system behaviour.
#[cfg_attr(alef, alef(skip))]
pub struct GuardedResolver;

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
mod resolver_impl {
    use std::sync::Arc;

    use reqwest::dns::{Addrs, Name, Resolve, Resolving};

    use super::{GuardedResolver, OutboundPolicy, current_policy, is_forbidden};

    impl Resolve for GuardedResolver {
        fn resolve(&self, name: Name) -> Resolving {
            Box::pin(async move {
                let policy = current_policy();
                let host = name.as_str().to_string();

                let addrs: Vec<_> = tokio::net::lookup_host(format!("{host}:0"))
                    .await
                    .map_err(|e| {
                        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                        err
                    })?
                    .collect();

                if !matches!(policy, OutboundPolicy::Off) {
                    for sa in &addrs {
                        if is_forbidden(sa.ip()) {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!(
                                "outbound DNS resolution for '{host}' produced \
                                 forbidden address {}",
                                sa.ip()
                            )
                            .into();
                            return Err(err);
                        }
                    }
                }

                let iter: Addrs = Box::new(addrs.into_iter());
                Ok(iter)
            })
        }
    }

    /// Build an [`Arc`]-wrapped [`GuardedResolver`] ready for use with
    /// `reqwest::Client::builder().dns_resolver(...)`.
    #[cfg_attr(alef, alef(skip))]
    pub fn guarded_resolver() -> Arc<GuardedResolver> {
        Arc::new(GuardedResolver)
    }
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub use resolver_impl::guarded_resolver;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    /// Helper for sync tests — `#[serial(outbound_policy)]` on the test fn
    /// guarantees no other policy-mutating test is running concurrently.
    fn with_policy<F: FnOnce()>(policy: OutboundPolicy, f: F) {
        set_outbound_policy(policy);
        f();
        set_outbound_policy(OutboundPolicy::Off);
    }

    // ── is_forbidden table tests ──────────────────────────────────────────────

    #[test]
    fn is_forbidden_recognizes_private_ranges() {
        let cases: &[(&str, bool)] = &[
            ("10.0.0.1", true),
            ("172.16.0.1", true),
            ("192.168.1.1", true),
            ("127.0.0.1", true),
            ("169.254.0.1", true),
            ("100.100.0.1", true),     // CGNAT
            ("0.0.0.0", true),         // unspecified
            ("255.255.255.255", true), // broadcast
            ("224.0.0.1", true),       // multicast
            ("8.8.8.8", false),        // public DNS — allowed
            ("1.1.1.1", false),        // Cloudflare — allowed
        ];
        for (addr, expected) in cases {
            let ip: IpAddr = addr.parse().expect("valid IP");
            assert_eq!(is_forbidden(ip), *expected, "is_forbidden({addr}) should be {expected}");
        }
    }

    #[test]
    fn is_forbidden_ipv6_loopback() {
        let ip: IpAddr = "::1".parse().unwrap();
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_ula() {
        let ip: IpAddr = "fc00::1".parse().unwrap();
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_link_local() {
        let ip: IpAddr = "fe80::1".parse().unwrap();
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_public() {
        let ip: IpAddr = "2001:4860:4860::8888".parse().unwrap(); // Google DNS
        assert!(!is_forbidden(ip));
    }

    // ── validate_outbound_url_sync ────────────────────────────────────────────
    //
    // All policy-mutating tests share `#[serial(outbound_policy)]` so they
    // never run concurrently with each other or with the async tests below.

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_off_passes_everything() {
        with_policy(OutboundPolicy::Off, || {
            assert!(validate_outbound_url_sync("http://127.0.0.1/").is_ok());
            assert!(validate_outbound_url_sync("http://169.254.169.254/").is_ok());
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_deny_private_rejects_loopback() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let result = validate_outbound_url_sync("http://127.0.0.1/");
            assert!(result.is_err(), "loopback should be rejected");
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("forbidden"),
                "error message should mention 'forbidden': {err}"
            );
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_deny_private_rejects_metadata_ip() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let result = validate_outbound_url_sync("http://169.254.169.254/");
            assert!(result.is_err(), "metadata IP should be rejected");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_deny_private_rejects_ula() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let result = validate_outbound_url_sync("http://[fc00::1]/");
            assert!(result.is_err(), "ULA address should be rejected");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_deny_private_rejects_link_local_v6() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let result = validate_outbound_url_sync("http://[fe80::1]/");
            assert!(result.is_err(), "IPv6 link-local should be rejected");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_deny_private_rejects_unknown_scheme() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let result = validate_outbound_url_sync("ftp://example.com/");
            assert!(result.is_err(), "ftp:// scheme should be rejected");
            let err = result.unwrap_err().to_string();
            assert!(err.contains("scheme"), "error should mention 'scheme': {err}");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_allowlist_accepts_exact_origin() {
        let allowed = vec![Url::parse("https://api.openai.com").unwrap()];
        with_policy(OutboundPolicy::Allowlist(allowed), || {
            let result = validate_outbound_url_sync("https://api.openai.com/v1/chat/completions");
            assert!(result.is_ok(), "same-origin with different path should pass");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_allowlist_rejects_other_host() {
        let allowed = vec![Url::parse("https://api.openai.com").unwrap()];
        with_policy(OutboundPolicy::Allowlist(allowed), || {
            let result = validate_outbound_url_sync("https://api.anthropic.com/");
            assert!(result.is_err(), "different host should be rejected");
        });
    }

    // ── validate_outbound_url (async, DNS) ────────────────────────────────────

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_off_passes_everything() {
        set_outbound_policy(OutboundPolicy::Off);
        assert!(validate_outbound_url("http://127.0.0.1/").await.is_ok());
        assert!(validate_outbound_url("http://169.254.169.254/").await.is_ok());
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_deny_private_rejects_loopback() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = validate_outbound_url("http://127.0.0.1/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "loopback should be rejected by DenyPrivate");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_deny_private_rejects_metadata_ip() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = validate_outbound_url("http://169.254.169.254/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "AWS metadata IP should be rejected");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_deny_private_rejects_ula() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = validate_outbound_url("http://[fc00::1]/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "ULA address should be rejected");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_deny_private_rejects_link_local_v6() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = validate_outbound_url("http://[fe80::1]/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "IPv6 link-local should be rejected");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_deny_private_rejects_unknown_scheme() {
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let result = validate_outbound_url("ftp://example.com/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "ftp:// scheme should be rejected");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_allowlist_accepts_exact_origin() {
        let allowed = vec![Url::parse("https://api.openai.com").unwrap()];
        set_outbound_policy(OutboundPolicy::Allowlist(allowed));
        let result = validate_outbound_url("https://api.openai.com/v1/chat/completions").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_ok(), "same-origin with different path should pass");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_allowlist_rejects_other_host() {
        let allowed = vec![Url::parse("https://api.openai.com").unwrap()];
        set_outbound_policy(OutboundPolicy::Allowlist(allowed));
        let result = validate_outbound_url("https://api.anthropic.com/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "different host should be rejected");
    }
}
