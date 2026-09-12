//! Outbound HTTP request policy guard.
//!
//! Allows callers (the proxy server in particular) to constrain which upstream
//! URLs can be reached.  The library default is [`OutboundPolicy::Off`] —
//! preserves backward-compatibility for FFI consumers and embedded applications
//! where the application owner registers their own providers and is trusted.
//! The proxy switches the policy to [`OutboundPolicy::DenyPrivate`] at startup
//! so multi-tenant deployments cannot SSRF into cloud metadata services or
//! private networks. Native HTTP clients enforce the policy before each
//! request, on redirects, and again after connection-time DNS resolution.

#[cfg(all(any(feature = "native-http", feature = "wasm-http"), not(target_arch = "wasm32")))]
use std::error::Error as _;
use std::net::IpAddr;
use std::sync::{OnceLock, RwLock};

use url::Url;

use crate::error::LiterLlmError;

/// Controls which upstream URLs the library is allowed to connect to.
///
/// Set once at application startup via [`set_outbound_policy`].  Checked at
/// provider registration time, immediately before each effective request, on
/// native redirect hops, and at every native TCP connection attempt (defense
/// in depth against DNS rebinding).
#[derive(Debug, Clone, Default)]
#[cfg_attr(alef, alef(skip))]
pub enum OutboundPolicy {
    /// No restrictions — library default.  Use only when the application is
    /// the sole registrar of provider URLs and trusts itself.
    #[default]
    Off,

    /// Reject URLs whose host resolves to any private / loopback / link-local
    /// / multicast / CGNAT address. Recommended for native multi-tenant proxies.
    /// WASM can reject literal addresses but browser-owned DNS and redirects
    /// cannot be inspected by this policy.
    DenyPrivate,

    /// Only allow URLs whose origin (scheme + host + port) matches one of the
    /// provided entries. Allowlisted hostnames may resolve to any address,
    /// including private, loopback, and link-local addresses. Only allowlist
    /// hostnames whose DNS is trusted: this mode does not prevent an allowed
    /// hostname from rebinding into a private network. Literal forbidden IP
    /// addresses remain rejected by URL validation.
    Allowlist(Vec<Url>),
}

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
#[tracing::instrument(level = "info")]
pub fn set_outbound_policy(policy: OutboundPolicy) {
    // ~keep A poisoned lock must not permanently disable outbound-policy enforcement:
    // recover the guard and keep going rather than propagate the panic.
    let mut guard = policy_lock().write().unwrap_or_else(|poisoned| {
        tracing::warn!("outbound policy lock poisoned; recovering");
        poisoned.into_inner()
    });
    *guard = policy;
}

/// Read a snapshot of the current outbound policy.
#[cfg_attr(alef, alef(skip))]
pub fn current_policy() -> OutboundPolicy {
    // ~keep See `set_outbound_policy`: recover rather than panic on a poisoned lock.
    policy_lock()
        .read()
        .unwrap_or_else(|poisoned| {
            tracing::warn!("outbound policy lock poisoned; recovering");
            poisoned.into_inner()
        })
        .clone()
}

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

    validate_literal_host(&url, raw_url)?;

    match policy {
        OutboundPolicy::Off => Ok(()),
        OutboundPolicy::DenyPrivate => check_deny_private(&url, raw_url).await,
        OutboundPolicy::Allowlist(allowed) => check_allowlist(&url, raw_url, &allowed),
    }
}

#[cfg(target_arch = "wasm32")]
async fn check_deny_private(_url: &Url, _raw: &str) -> Result<(), LiterLlmError> {
    // ~keep Literal private hosts are rejected before this function. WASM has no Rust DNS or
    // ~keep redirect hook, so hostname resolution and redirect enforcement remain browser-owned.
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

    validate_literal_host(&url, raw_url)?;

    if let OutboundPolicy::Allowlist(allowed) = policy {
        return check_allowlist(&url, raw_url, &allowed);
    }

    Ok(())
}

fn validate_literal_host(url: &Url, raw_url: &str) -> Result<(), LiterLlmError> {
    match url.host() {
        Some(url::Host::Ipv4(v4)) if is_forbidden(IpAddr::V4(v4)) => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_owned(),
                reason: format!("host is a forbidden address {v4}"),
            });
        }
        Some(url::Host::Ipv6(v6)) if is_forbidden(IpAddr::V6(v6)) => {
            return Err(LiterLlmError::OutboundForbidden {
                url: raw_url.to_owned(),
                reason: format!("host is a forbidden address {v6}"),
            });
        }
        _ => {}
    }
    Ok(())
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
fn outbound_redirect_policy(allow_cross_origin: bool) -> reqwest::redirect::Policy {
    let default_policy = reqwest::redirect::Policy::default();
    reqwest::redirect::Policy::custom(move |attempt| {
        match validate_redirect(attempt.previous().last(), attempt.url(), allow_cross_origin) {
            Ok(()) => default_policy.redirect(attempt),
            Err(error) => attempt.error(error),
        }
    })
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
fn validate_redirect(previous: Option<&Url>, next: &Url, allow_cross_origin: bool) -> Result<(), LiterLlmError> {
    if let Some(previous) = previous
        && previous.scheme() == "https"
        && next.scheme() == "http"
    {
        return Err(LiterLlmError::OutboundForbidden {
            url: next.as_str().to_owned(),
            reason: "HTTPS-to-HTTP redirects are forbidden".into(),
        });
    }

    if matches!(current_policy(), OutboundPolicy::Off) {
        return Ok(());
    }

    if let Some(previous) = previous
        && !allow_cross_origin
        && !same_origin(previous, next)
    {
        return Err(LiterLlmError::OutboundForbidden {
            url: next.as_str().to_owned(),
            reason: "cross-origin redirects are forbidden while an outbound policy is active".into(),
        });
    }

    validate_outbound_url_sync(next.as_str())
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
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
    let origin_match = allowed.iter().any(|allowed_url| same_origin(allowed_url, url));
    if origin_match {
        Ok(())
    } else {
        Err(LiterLlmError::OutboundForbidden {
            url: raw.to_string(),
            reason: "URL not in outbound allowlist".into(),
        })
    }
}

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
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use reqwest::dns::{Addrs, Name, Resolve, Resolving};

    use super::{GuardedResolver, LiterLlmError, OutboundPolicy, current_policy, is_forbidden};

    #[derive(Clone)]
    struct DnsCacheEntry {
        expires_at: Instant,
        addrs: Vec<SocketAddr>,
    }

    struct CachedGuardedResolver {
        cache_ttl: Option<Duration>,
        cache: Arc<Mutex<HashMap<String, DnsCacheEntry>>>,
    }

    impl CachedGuardedResolver {
        fn new(cache_ttl: Option<Duration>) -> Self {
            Self {
                cache_ttl,
                cache: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    fn cached_addrs(
        cache: &Mutex<HashMap<String, DnsCacheEntry>>,
        cache_ttl: Option<Duration>,
        host: &str,
    ) -> Option<Vec<SocketAddr>> {
        cache_ttl?;

        let now = Instant::now();
        let mut guard = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = guard.get(host)?;
        if entry.expires_at > now {
            return Some(entry.addrs.clone());
        }
        guard.remove(host);
        None
    }

    fn store_addrs(
        cache: &Mutex<HashMap<String, DnsCacheEntry>>,
        cache_ttl: Option<Duration>,
        host: String,
        addrs: Vec<SocketAddr>,
    ) {
        let Some(ttl) = cache_ttl else {
            return;
        };
        if ttl.is_zero() {
            return;
        }

        let expires_at = Instant::now().checked_add(ttl).unwrap_or_else(Instant::now);
        let mut guard = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.insert(host, DnsCacheEntry { expires_at, addrs });
    }

    async fn resolve_system(host: &str) -> Result<Vec<SocketAddr>, Box<dyn std::error::Error + Send + Sync>> {
        let addrs = tokio::net::lookup_host((host, 0))
            .await
            .map_err(|e| {
                let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                err
            })?
            .collect();
        Ok(addrs)
    }

    pub(super) fn validate_addrs(
        policy: OutboundPolicy,
        host: &str,
        addrs: &[SocketAddr],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // DNS only exposes the hostname, so callers must validate the initial
        // URL's full origin separately. Trust private DNS answers only for an
        // explicitly listed hostname, including when rechecking cached answers.
        match policy {
            OutboundPolicy::Off => return Ok(()),
            OutboundPolicy::Allowlist(allowed) if allowed.iter().any(|url| url.host_str() == Some(host)) => {
                return Ok(());
            }
            _ => {}
        }

        for sa in addrs {
            if is_forbidden(sa.ip()) {
                let err: Box<dyn std::error::Error + Send + Sync> = Box::new(LiterLlmError::OutboundForbidden {
                    url: format!("dns://{host}"),
                    reason: format!("DNS resolution produced forbidden address {}", sa.ip()),
                });
                return Err(err);
            }
        }

        Ok(())
    }

    impl Resolve for GuardedResolver {
        fn resolve(&self, name: Name) -> Resolving {
            Box::pin(async move {
                let policy = current_policy();
                let host = name.as_str().to_string();

                let addrs = resolve_system(&host).await?;
                validate_addrs(policy, &host, &addrs)?;

                let iter: Addrs = Box::new(addrs.into_iter());
                Ok(iter)
            })
        }
    }

    impl Resolve for CachedGuardedResolver {
        fn resolve(&self, name: Name) -> Resolving {
            let policy = current_policy();
            let host = name.as_str().to_string();
            let cache_ttl = self.cache_ttl;
            let cache = Arc::clone(&self.cache);

            Box::pin(async move {
                if let Some(addrs) = cached_addrs(&cache, cache_ttl, &host) {
                    validate_addrs(policy, &host, &addrs)?;
                    let iter: Addrs = Box::new(addrs.into_iter());
                    return Ok(iter);
                }

                let addrs = resolve_system(&host).await?;
                validate_addrs(policy, &host, &addrs)?;
                store_addrs(&cache, cache_ttl, host, addrs.clone());

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

    /// Build a resolver that applies the active outbound policy and optionally
    /// caches successful DNS lookups for the configured TTL.
    #[cfg_attr(alef, alef(skip))]
    pub fn cached_guarded_resolver(cache_ttl: Option<Duration>) -> Arc<dyn Resolve> {
        Arc::new(CachedGuardedResolver::new(cache_ttl))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::provider::outbound_policy::set_outbound_policy;
        use serial_test::serial;
        use url::Url;

        #[tokio::test]
        #[serial(outbound_policy)]
        async fn cached_private_addresses_follow_current_policy() {
            let resolver = CachedGuardedResolver::new(Some(Duration::from_secs(60)));
            let addresses = vec![
                "172.18.0.2:0".parse().expect("private IPv4"),
                "[fd00::2]:0".parse().expect("private IPv6"),
            ];
            store_addrs(
                &resolver.cache,
                resolver.cache_ttl,
                "internal-llm.invalid".into(),
                addresses.clone(),
            );
            set_outbound_policy(OutboundPolicy::Allowlist(vec![
                Url::parse("http://internal-llm.invalid:8000").expect("allowlisted origin"),
            ]));
            let allowed = resolver
                .resolve("internal-llm.invalid".parse().expect("DNS name"))
                .await;
            set_outbound_policy(OutboundPolicy::Allowlist(vec![
                Url::parse("http://other-service.invalid:8000").expect("different origin"),
            ]));
            let unlisted = resolver
                .resolve("internal-llm.invalid".parse().expect("DNS name"))
                .await;
            set_outbound_policy(OutboundPolicy::DenyPrivate);
            let denied = resolver
                .resolve("internal-llm.invalid".parse().expect("DNS name"))
                .await;
            set_outbound_policy(OutboundPolicy::Off);
            assert_eq!(
                allowed.expect("allowlisted cached addresses").collect::<Vec<_>>(),
                addresses
            );
            let error = unlisted
                .err()
                .expect("Allowlist must recheck cached hostname membership");
            assert!(matches!(
                error.downcast_ref::<LiterLlmError>(),
                Some(LiterLlmError::OutboundForbidden { .. })
            ));
            let error = denied.err().expect("DenyPrivate must recheck cached addresses");
            assert!(matches!(
                error.downcast_ref::<LiterLlmError>(),
                Some(LiterLlmError::OutboundForbidden { .. })
            ));
        }
    }
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub use resolver_impl::{cached_guarded_resolver, guarded_resolver};

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
/// Apply authenticated outbound-request protections to a native HTTP client.
///
/// The returned builder validates every resolved address and redirect target.
/// Cross-origin redirects are rejected so credentials in custom headers cannot
/// be forwarded to a different origin. Active policies also disable proxies;
/// otherwise the proxy could resolve the target outside the guarded resolver.
///
/// Callers must validate each initial request URL with [`validate_outbound_url_sync`]
/// or [`validate_outbound_url`] before sending it. The DNS resolver sees only the
/// hostname, not the scheme or port, and literal IP URLs bypass DNS entirely.
/// These hooks therefore do not enforce the complete initial-origin policy.
#[cfg_attr(alef, alef(skip))]
pub fn configure_outbound_client_builder(
    mut builder: reqwest::ClientBuilder,
    dns_cache_ttl: Option<std::time::Duration>,
) -> reqwest::ClientBuilder {
    builder = builder.redirect(outbound_redirect_policy(false));
    if !matches!(current_policy(), OutboundPolicy::Off) {
        builder = builder.no_proxy();
        builder = builder.dns_resolver(cached_guarded_resolver(dns_cache_ttl));
    } else if dns_cache_ttl.is_some() {
        builder = builder.dns_resolver(cached_guarded_resolver(dns_cache_ttl));
    }
    builder
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub(crate) fn configure_credential_free_outbound_client_builder(
    mut builder: reqwest::ClientBuilder,
) -> reqwest::ClientBuilder {
    builder = builder.redirect(outbound_redirect_policy(true));
    if !matches!(current_policy(), OutboundPolicy::Off) {
        builder = builder.no_proxy();
        builder = builder.dns_resolver(cached_guarded_resolver(None));
    }
    builder
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
pub(crate) fn authenticated_outbound_client() -> Result<reqwest::Client, LiterLlmError> {
    crate::ensure_crypto_provider();
    configure_outbound_client_builder(reqwest::Client::builder(), None)
        .build()
        .map_err(LiterLlmError::from)
}

#[cfg(all(any(feature = "native-http", feature = "wasm-http"), not(target_arch = "wasm32")))]
pub(crate) fn outbound_forbidden_from_reqwest(error: &reqwest::Error) -> Option<LiterLlmError> {
    let mut source = error.source();
    while let Some(current) = source {
        if let Some(LiterLlmError::OutboundForbidden { url, reason }) = current.downcast_ref::<LiterLlmError>() {
            return Some(LiterLlmError::OutboundForbidden {
                url: url.clone(),
                reason: reason.clone(),
            });
        }
        source = current.source();
    }
    None
}

#[cfg(all(feature = "wasm-http", target_arch = "wasm32"))]
pub(crate) fn outbound_forbidden_from_reqwest(_error: &reqwest::Error) -> Option<LiterLlmError> {
    // ~keep Browser fetch owns DNS and redirect handling, so reqwest cannot carry a native
    // ~keep guarded-resolver or redirect-policy error on WASM.
    None
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    use std::io::{Read, Write};
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    use std::net::{SocketAddr, TcpListener};
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    use std::sync::Arc;
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    use std::sync::atomic::{AtomicBool, Ordering};
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    use std::time::{Duration, Instant};

    use serial_test::serial;

    use super::*;

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    struct OneShotServer {
        address: SocketAddr,
        hit: Arc<AtomicBool>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    impl OneShotServer {
        fn start(response: String) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect test server");
            listener.set_nonblocking(true).expect("set redirect server nonblocking");
            let address = listener.local_addr().expect("redirect server address");
            let hit = Arc::new(AtomicBool::new(false));
            let hit_writer = Arc::clone(&hit);
            let handle = std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(2);
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            hit_writer.store(true, Ordering::SeqCst);
                            stream.set_nonblocking(false).expect("set redirect stream blocking");
                            let mut request = [0_u8; 4096];
                            let _ = stream.read(&mut request).expect("read redirect request");
                            stream.write_all(response.as_bytes()).expect("write redirect response");
                            break;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("accept redirect request: {error}"),
                    }
                }
            });
            Self {
                address,
                hit,
                handle: Some(handle),
            }
        }

        fn finish(mut self) -> bool {
            self.handle
                .take()
                .expect("redirect server handle")
                .join()
                .expect("redirect server thread");
            self.hit.load(Ordering::SeqCst)
        }
    }

    /// Helper for sync tests — `#[serial(outbound_policy)]` on the test fn
    /// guarantees no other policy-mutating test is running concurrently.
    fn with_policy<F: FnOnce()>(policy: OutboundPolicy, f: F) {
        set_outbound_policy(policy);
        f();
        set_outbound_policy(OutboundPolicy::Off);
    }

    #[test]
    fn is_forbidden_recognizes_private_ranges() {
        let cases: &[(&str, bool)] = &[
            ("10.0.0.1", true),
            ("172.16.0.1", true),
            ("192.168.1.1", true),
            ("127.0.0.1", true),
            ("169.254.0.1", true),
            ("100.100.0.1", true),
            ("0.0.0.0", true),
            ("255.255.255.255", true),
            ("224.0.0.1", true),
            ("8.8.8.8", false),
            ("1.1.1.1", false),
        ];
        for (addr, expected) in cases {
            let ip: IpAddr = addr.parse().expect("valid IP");
            assert_eq!(is_forbidden(ip), *expected, "is_forbidden({addr}) should be {expected}");
        }
    }

    #[test]
    fn is_forbidden_ipv6_loopback() {
        let ip: IpAddr = "::1".parse().expect("::1 is a valid IPv6 address");
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_ula() {
        let ip: IpAddr = "fc00::1".parse().expect("fc00::1 is a valid IPv6 address");
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_link_local() {
        let ip: IpAddr = "fe80::1".parse().expect("fe80::1 is a valid IPv6 address");
        assert!(is_forbidden(ip));
    }

    #[test]
    fn is_forbidden_ipv6_public() {
        let ip: IpAddr = "2001:4860:4860::8888"
            .parse()
            .expect("Google DNS is a valid IPv6 address");
        assert!(!is_forbidden(ip));
    }

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
        let allowed = vec![Url::parse("https://api.openai.com").expect("openai URL should be valid")];
        with_policy(OutboundPolicy::Allowlist(allowed), || {
            let result = validate_outbound_url_sync("https://api.openai.com/v1/chat/completions");
            assert!(result.is_ok(), "same-origin with different path should pass");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_allowlist_rejects_other_host() {
        let allowed = vec![Url::parse("https://api.openai.com").expect("openai URL should be valid")];
        with_policy(OutboundPolicy::Allowlist(allowed), || {
            let result = validate_outbound_url_sync("https://api.anthropic.com/");
            assert!(result.is_err(), "different host should be rejected");
        });
    }

    #[test]
    #[serial(outbound_policy)]
    fn validate_sync_allowlist_requires_exact_scheme_host_and_effective_port() {
        let allowed = vec![Url::parse("https://api.example.com").expect("allowlist URL")];
        with_policy(OutboundPolicy::Allowlist(allowed), || {
            assert!(validate_outbound_url_sync("https://api.example.com:443/v1").is_ok());
            for rejected in [
                "http://api.example.com/v1",
                "https://other.example.com/v1",
                "https://api.example.com:8443/v1",
            ] {
                assert!(
                    validate_outbound_url_sync(rejected).is_err(),
                    "allowlist must reject a different origin: {rejected}"
                );
            }
        });
    }

    #[test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn redirect_validation_rejects_cross_origin_under_deny_private() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let previous = Url::parse("https://api.example.com/v1/chat").expect("previous URL");
            let next = Url::parse("https://cdn.example.net/v1/chat").expect("next URL");
            let result = validate_redirect(Some(&previous), &next, false);
            assert!(
                matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
                "active policies must reject cross-origin redirects that could leak custom auth headers"
            );
        });
    }

    #[test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn redirect_validation_allows_same_origin_under_deny_private() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let previous = Url::parse("https://api.example.com/v1/chat").expect("previous URL");
            let next = Url::parse("https://api.example.com/v2/chat").expect("next URL");
            assert!(validate_redirect(Some(&previous), &next, false).is_ok());
        });
    }

    #[test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn credential_free_redirect_still_rejects_private_target() {
        with_policy(OutboundPolicy::DenyPrivate, || {
            let previous = Url::parse("https://github.com/example/catalog.json").expect("previous URL");
            let next = Url::parse("http://169.254.169.254/catalog.json").expect("next URL");
            let result = validate_redirect(Some(&previous), &next, true);
            assert!(
                matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
                "credential-free redirects must still validate the target policy"
            );
        });
    }

    #[test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn credential_free_redirect_rejects_https_to_http_downgrade_when_policy_is_off() {
        with_policy(OutboundPolicy::Off, || {
            let previous = Url::parse("https://github.com/example/catalog.json").expect("previous URL");
            let next = Url::parse("http://release-assets.example.com/catalog.json").expect("next URL");
            let result = validate_redirect(Some(&previous), &next, true);
            assert!(
                matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
                "catalog redirects must never downgrade transport security"
            );
        });
    }

    #[test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn resolver_allowlist_accepts_private_address_after_origin_check() {
        let origin = Url::parse("http://internal-llm:8000").expect("valid origin");
        let policy = OutboundPolicy::Allowlist(vec![origin]);
        with_policy(policy.clone(), || {
            assert!(validate_outbound_url_sync("http://internal-llm:8000/v1/embeddings").is_ok());
            for denied in [
                "http://other-service:8000/v1/embeddings",
                "http://internal-llm:8001/v1/embeddings",
                "https://internal-llm:8000/v1/embeddings",
            ] {
                assert!(matches!(
                    validate_outbound_url_sync(denied),
                    Err(LiterLlmError::OutboundForbidden { .. })
                ));
            }
            let addrs = [
                "172.18.0.2:8000".parse().expect("private IPv4"),
                "[fd00::2]:8000".parse().expect("private IPv6"),
                "[::1]:8000".parse().expect("loopback IPv6"),
                "[fe80::2]:8000".parse().expect("link-local IPv6"),
            ];
            assert!(resolver_impl::validate_addrs(policy, "internal-llm", &addrs).is_ok());
        });
    }

    #[test]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn resolver_allowlist_rejects_unlisted_private_addresses() {
        let policy = OutboundPolicy::Allowlist(vec![Url::parse("http://internal-llm:8000").expect("origin")]);
        for host in ["other-service", "internal-llm.attacker.invalid", "internal-llm."] {
            for address in ["172.18.0.2:0", "[fd00::2]:0", "[::1]:0", "169.254.169.254:0"] {
                let error = resolver_impl::validate_addrs(policy.clone(), host, &[address.parse().expect("address")])
                    .expect_err("unlisted hostname must not bypass private-address filtering");
                assert!(matches!(
                    error.downcast_ref::<LiterLlmError>(),
                    Some(LiterLlmError::OutboundForbidden { .. })
                ));
            }
        }
    }

    #[test]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn resolver_deny_private_still_rejects_private_address() {
        let addrs = [
            "172.18.0.2:8000".parse().expect("private IPv4"),
            "[fd00::2]:8000".parse().expect("private IPv6"),
            "[::1]:8000".parse().expect("loopback IPv6"),
            "[fe80::2]:8000".parse().expect("link-local IPv6"),
        ];
        for address in addrs {
            let error = resolver_impl::validate_addrs(OutboundPolicy::DenyPrivate, "private-service", &[address])
                .expect_err("DenyPrivate must reject private DNS results");
            assert!(matches!(
                error.downcast_ref::<LiterLlmError>(),
                Some(LiterLlmError::OutboundForbidden { .. })
            ));
        }
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn configured_builder_allows_allowlisted_private_hostname() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://localhost:{}/v1/models", target.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&target_url).expect("allowlisted origin"),
        ]));
        let client = configure_outbound_client_builder(reqwest::Client::builder(), None)
            .build()
            .expect("guarded client");
        let result = crate::http::request::get_json_raw(&client, &target_url, None, &[], 0).await;
        set_outbound_policy(OutboundPolicy::Off);
        assert_eq!(
            result.expect("allowlisted private hostname must be reachable"),
            serde_json::json!({})
        );
        assert!(target.finish(), "allowlisted target must receive a request");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn configured_builder_direct_request_checks_private_hostname_membership() {
        for allow_target in [false, true] {
            let target = OneShotServer::start("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_string());
            let target_url = format!("http://localhost:{}/", target.address.port());
            let allowed_url = if allow_target {
                target_url.as_str()
            } else {
                "http://allowed.invalid:8000"
            };
            set_outbound_policy(OutboundPolicy::Allowlist(vec![
                Url::parse(allowed_url).expect("allowlisted origin"),
            ]));
            let client = configure_outbound_client_builder(reqwest::Client::builder(), None)
                .build()
                .expect("guarded client");
            // Exercise the exported builder without the URL-validating request wrapper.
            let result = client.get(&target_url).send().await;
            set_outbound_policy(OutboundPolicy::Off);
            let received = target.finish();
            if allow_target {
                assert_eq!(result.expect("allowlisted hostname must be reachable").status(), 200);
                assert!(received, "allowlisted target must receive a request");
            } else {
                let error = LiterLlmError::from(result.expect_err("unlisted localhost must be blocked"));
                assert!(matches!(error, LiterLlmError::OutboundForbidden { .. }));
                assert!(!received, "unlisted target must not receive a request");
            }
        }
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn allowlisted_private_origin_still_rejects_cross_origin_redirect() {
        let target = OneShotServer::start("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}".to_string());
        let target_url = format!("http://localhost:{}/done", target.address.port());
        let source = OneShotServer::start(format!(
            "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\n\r\n"
        ));
        let source_url = format!("http://localhost:{}/start", source.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&source_url).expect("source origin"),
            Url::parse(&target_url).expect("target origin"),
        ]));
        let client = configure_outbound_client_builder(reqwest::Client::builder(), None)
            .build()
            .expect("guarded client");
        let result = crate::http::request::get_json_raw(&client, &source_url, None, &[], 0).await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(matches!(result, Err(LiterLlmError::OutboundForbidden { .. })));
        assert!(source.finish(), "allowed private source must receive the request");
        assert!(!target.finish(), "different-origin target must not receive the request");
    }

    #[test]
    #[serial(outbound_policy)]
    fn allowlist_still_rejects_literal_private_addresses() {
        for url in ["http://127.0.0.1:8000", "http://[::1]:8000", "http://169.254.169.254"] {
            with_policy(
                OutboundPolicy::Allowlist(vec![Url::parse(url).expect("origin")]),
                || {
                    assert!(matches!(
                        validate_outbound_url_sync(url),
                        Err(LiterLlmError::OutboundForbidden { .. })
                    ));
                },
            );
        }
    }

    #[test]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    fn guarded_resolver_rejects_private_connection_address() {
        let addresses = [SocketAddr::from(([127, 0, 0, 1], 443))];
        let result = resolver_impl::validate_addrs(OutboundPolicy::DenyPrivate, "api.example.com", &addresses);
        assert!(
            result.is_err(),
            "connection-time DNS results must reject private addresses"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn configured_builder_rejects_private_hostname_resolution() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://localhost:{}/", target.address.port());
        set_outbound_policy(OutboundPolicy::DenyPrivate);
        let client = configure_outbound_client_builder(reqwest::Client::builder(), None)
            .build()
            .expect("guarded client");

        let result = client.get(&target_url).send().await;
        set_outbound_policy(OutboundPolicy::Off);

        let error = LiterLlmError::from(result.expect_err("guarded builder must reject localhost DNS"));
        assert!(
            matches!(error, LiterLlmError::OutboundForbidden { .. }),
            "resolver policy failures must preserve OutboundForbidden: {error:?}"
        );
        assert!(!target.finish(), "a DNS-blocked target must not receive a request");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn credential_free_client_allows_policy_checked_cross_origin_redirect() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://catalog-assets.test:{}/catalog.json", target.address.port());
        let source = OneShotServer::start(format!(
            "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\n\r\n"
        ));
        let source_url = format!("http://catalog-origin.test:{}/catalog.json", source.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&source_url).expect("source allowlist URL"),
            Url::parse(&target_url).expect("target allowlist URL"),
        ]));
        let client = reqwest::Client::builder()
            .resolve("catalog-origin.test", source.address)
            .resolve("catalog-assets.test", target.address)
            .redirect(outbound_redirect_policy(true))
            .build()
            .expect("credential-free redirect client");

        let result = crate::http::request::get_json_raw(&client, &source_url, None, &[], 0).await;
        set_outbound_policy(OutboundPolicy::Off);

        assert_eq!(result.expect("allowlisted catalog redirect"), serde_json::json!({}));
        assert!(source.finish(), "catalog origin must receive the initial request");
        assert!(
            target.finish(),
            "catalog asset origin must receive the redirected request"
        );
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn redirect_policy_failure_is_outbound_forbidden_and_not_retried() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://blocked.test:{}/done", target.address.port());
        let source = OneShotServer::start(format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: {target_url}\r\nContent-Length: 0\r\n\r\n"
        ));
        let source_url = format!("http://allowed.test:{}/start", source.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&source_url).expect("source allowlist URL"),
        ]));
        let client = reqwest::Client::builder()
            .resolve("allowed.test", source.address)
            .resolve("blocked.test", target.address)
            .redirect(outbound_redirect_policy(false))
            .build()
            .expect("redirect test client");
        let mut attempts = 0_u32;

        let result = crate::http::request::with_retry(&source_url, 3, || {
            attempts += 1;
            client
                .post(&source_url)
                .header("authorization", "Bearer secret-token")
                .header("x-api-key", "secret-api-key")
                .body("secret request body")
                .send()
        })
        .await;
        set_outbound_policy(OutboundPolicy::Off);

        assert!(
            matches!(result, Err(LiterLlmError::OutboundForbidden { .. })),
            "redirect policy errors must preserve their classification: {result:?}"
        );
        assert_eq!(attempts, 1, "policy failures are deterministic and must not be retried");
        assert!(
            source.finish(),
            "allowed source must receive exactly the initial request"
        );
        assert!(!target.finish(), "blocked redirect target must not receive a request");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn redirect_policy_rejects_allowed_to_unallowed_public_origin() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://unallowed.test:{}/done", target.address.port());
        let source = OneShotServer::start(format!(
            "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\n\r\n"
        ));
        let source_url = format!("http://allowed.test:{}/start", source.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&source_url).expect("source allowlist URL"),
        ]));
        let client = reqwest::Client::builder()
            .resolve("allowed.test", source.address)
            .resolve("unallowed.test", target.address)
            .redirect(outbound_redirect_policy(false))
            .build()
            .expect("redirect test client");

        let result = crate::http::request::get_json_raw(&client, &source_url, None, &[], 0).await;
        set_outbound_policy(OutboundPolicy::Off);

        assert!(result.is_err(), "redirect outside the allowlist must fail");
        assert!(source.finish(), "allowed source must receive the initial request");
        assert!(!target.finish(), "unallowed redirect target must not receive a request");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    #[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
    async fn redirect_policy_rejects_allowed_to_private_origin() {
        let target = OneShotServer::start(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}".to_string(),
        );
        let target_url = format!("http://127.0.0.1:{}/done", target.address.port());
        let source = OneShotServer::start(format!(
            "HTTP/1.1 302 Found\r\nLocation: {target_url}\r\nContent-Length: 0\r\n\r\n"
        ));
        let source_url = format!("http://allowed.test:{}/start", source.address.port());
        set_outbound_policy(OutboundPolicy::Allowlist(vec![
            Url::parse(&source_url).expect("source allowlist URL"),
        ]));
        let client = reqwest::Client::builder()
            .resolve("allowed.test", source.address)
            .redirect(outbound_redirect_policy(false))
            .build()
            .expect("redirect test client");

        let result = crate::http::request::get_json_raw(&client, &source_url, None, &[], 0).await;
        set_outbound_policy(OutboundPolicy::Off);

        assert!(result.is_err(), "redirect into private address space must fail");
        assert!(source.finish(), "allowed source must receive the initial request");
        assert!(!target.finish(), "private redirect target must not receive a request");
    }

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
        let allowed = vec![Url::parse("https://api.openai.com").expect("openai URL should be valid")];
        set_outbound_policy(OutboundPolicy::Allowlist(allowed));
        let result = validate_outbound_url("https://api.openai.com/v1/chat/completions").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_ok(), "same-origin with different path should pass");
    }

    #[tokio::test]
    #[serial(outbound_policy)]
    async fn validate_async_allowlist_rejects_other_host() {
        let allowed = vec![Url::parse("https://api.openai.com").expect("openai URL should be valid")];
        set_outbound_policy(OutboundPolicy::Allowlist(allowed));
        let result = validate_outbound_url("https://api.anthropic.com/").await;
        set_outbound_policy(OutboundPolicy::Off);
        assert!(result.is_err(), "different host should be rejected");
    }
}
