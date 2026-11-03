//! Per-client blocked networks.
//!
//! A client built with [`ClientConfig::blocked_networks`](crate::ClientConfig::blocked_networks)
//! refuses every address inside these networks: at connect time for DNS answers, and before the
//! request is sent for an IP-literal host. The process-wide [`OutboundPolicy`](super::OutboundPolicy)
//! still applies on top.

use std::net::IpAddr;
use std::sync::Arc;

use crate::error::{LiterLlmError, Result};

/// One network in CIDR form, with host bits cleared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Network {
    address: IpAddr,
    prefix: u8,
}

impl Network {
    /// Parses `a.b.c.d/n`, `x::y/n` or a bare address (a single host).
    fn parse(text: &str) -> Option<Self> {
        let (address, prefix) = match text.trim().split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (text.trim(), None),
        };
        let address: IpAddr = address.parse().ok()?;
        let max = if address.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(prefix) if prefix.bytes().all(|byte| byte.is_ascii_digit()) => prefix.parse().ok()?,
            Some(_) => return None,
            None => max,
        };
        if prefix > max {
            return None;
        }
        Some(Self {
            address: mask(address, prefix),
            prefix,
        })
    }

    fn contains(&self, address: IpAddr) -> bool {
        match (self.address, address) {
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_)) => {
                mask(address, self.prefix) == self.address
            }
            _ => false,
        }
    }
}

fn mask(address: IpAddr, prefix: u8) -> IpAddr {
    match address {
        IpAddr::V4(v4) => {
            let bits = u32::from(v4);
            let kept = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V4((bits & kept).into())
        }
        IpAddr::V6(v6) => {
            let bits = u128::from(v6);
            let kept = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
            IpAddr::V6((bits & kept).into())
        }
    }
}

/// A parsed, shareable list of blocked networks.
#[derive(Debug, Clone)]
pub(crate) struct BlockedNetworks(Arc<[Network]>);

impl BlockedNetworks {
    /// Parses CIDR strings. Returns `Ok(None)` for an empty list.
    ///
    /// # Errors
    ///
    /// Returns [`LiterLlmError::BadRequest`] naming the first entry that is not an IP address or
    /// a CIDR network.
    pub(crate) fn parse(entries: &[String]) -> Result<Option<Self>> {
        if entries.is_empty() {
            return Ok(None);
        }
        let networks = entries
            .iter()
            .map(|entry| {
                Network::parse(entry).ok_or_else(|| LiterLlmError::BadRequest {
                    message: format!("blocked_networks entry {entry:?} is not an IP address or CIDR network"),
                    status: 400,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Some(Self(networks.into())))
    }

    /// Whether `address` lies in a blocked network. An IPv4-mapped IPv6 address
    /// (`::ffff:a.b.c.d`) is also matched as its IPv4 address.
    pub(crate) fn contains(&self, address: IpAddr) -> bool {
        let mapped = match address {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4),
            IpAddr::V4(_) => None,
        };
        self.0
            .iter()
            .any(|network| network.contains(address) || mapped.is_some_and(|v4| network.contains(v4)))
    }

    /// Refuses a URL whose host is an IP literal inside a blocked network.
    ///
    /// A hostname passes here; its addresses are checked when the client resolves it.
    ///
    /// # Errors
    ///
    /// Returns [`LiterLlmError::OutboundForbidden`] for a blocked IP-literal host.
    pub(crate) fn check_url(&self, raw_url: &str) -> Result<()> {
        let Ok(url) = url::Url::parse(raw_url) else {
            return Ok(());
        };
        let (address, label) = match url.host() {
            Some(url::Host::Ipv4(v4)) => (IpAddr::V4(v4), v4.to_string()),
            Some(url::Host::Ipv6(v6)) => (IpAddr::V6(v6), format!("[{v6}]")),
            _ => return Ok(()),
        };
        if !self.contains(address) {
            return Ok(());
        }
        let label = match url.port_or_known_default() {
            Some(port) => format!("{label}:{port}"),
            None => label,
        };
        Err(LiterLlmError::OutboundForbidden {
            url: label,
            reason: "address is in a blocked network for this client".into(),
        })
    }
}

#[cfg(all(feature = "native-http", not(target_arch = "wasm32")))]
mod resolver {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use reqwest::dns::{Addrs, Name, Resolve, Resolving};

    use super::BlockedNetworks;
    use crate::error::LiterLlmError;

    /// Wraps the client's resolver and refuses the lookup when any answer is blocked.
    ///
    /// Refusing the whole answer, not filtering it, keeps the result independent of
    /// which address the connector would try first.
    struct BlockingResolver {
        inner: Arc<dyn Resolve>,
        blocked: BlockedNetworks,
    }

    impl Resolve for BlockingResolver {
        fn resolve(&self, name: Name) -> Resolving {
            let host = name.as_str().to_owned();
            let lookup = self.inner.resolve(name);
            let blocked = self.blocked.clone();
            Box::pin(async move {
                let addrs: Vec<SocketAddr> = lookup.await?.collect();
                if let Some(address) = addrs.iter().map(SocketAddr::ip).find(|ip| blocked.contains(*ip)) {
                    let error: Box<dyn std::error::Error + Send + Sync> = Box::new(LiterLlmError::OutboundForbidden {
                        url: format!("dns://{host}"),
                        reason: format!(
                            "DNS resolution produced address {address} in a blocked network for this client"
                        ),
                    });
                    return Err(error);
                }
                let iter: Addrs = Box::new(addrs.into_iter());
                Ok(iter)
            })
        }
    }

    impl BlockedNetworks {
        /// Installs the blocking resolver over the guarded resolver and turns off proxies,
        /// because a proxy would resolve the host outside this resolver.
        pub(crate) fn apply_to_builder(
            &self,
            builder: reqwest::ClientBuilder,
            dns_cache_ttl: Option<std::time::Duration>,
        ) -> reqwest::ClientBuilder {
            let inner = crate::provider::outbound_policy::cached_guarded_resolver(dns_cache_ttl);
            builder.no_proxy().dns_resolver(Arc::new(BlockingResolver {
                inner,
                blocked: self.clone(),
            }))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::IpAddr;

        struct FixedResolver(Vec<SocketAddr>);

        impl Resolve for FixedResolver {
            fn resolve(&self, _name: Name) -> Resolving {
                let addrs = self.0.clone();
                Box::pin(async move {
                    let iter: Addrs = Box::new(addrs.into_iter());
                    Ok(iter)
                })
            }
        }

        fn resolver(answers: &[&str]) -> BlockingResolver {
            BlockingResolver {
                inner: Arc::new(FixedResolver(
                    answers
                        .iter()
                        .map(|ip| SocketAddr::new(ip.parse::<IpAddr>().expect("IP"), 0))
                        .collect(),
                )),
                blocked: BlockedNetworks::parse(&["10.0.0.0/8".into(), "fd00::/8".into()])
                    .expect("valid networks")
                    .expect("non-empty"),
            }
        }

        #[tokio::test]
        async fn lookup_with_one_blocked_answer_is_refused() {
            let result = resolver(&["8.8.8.8", "10.1.2.3"])
                .resolve("model.example".parse().expect("DNS name"))
                .await;
            let error = result.err().expect("a blocked answer must refuse the lookup");
            assert!(matches!(
                error.downcast_ref::<LiterLlmError>(),
                Some(LiterLlmError::OutboundForbidden { .. })
            ));
        }

        #[tokio::test]
        async fn lookup_with_only_allowed_answers_passes() {
            let addrs: Vec<_> = resolver(&["8.8.8.8", "2001:4860::1"])
                .resolve("model.example".parse().expect("DNS name"))
                .await
                .expect("allowed answers")
                .collect();
            assert_eq!(addrs.len(), 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(entries: &[&str]) -> BlockedNetworks {
        let entries: Vec<String> = entries.iter().map(|entry| (*entry).to_owned()).collect();
        BlockedNetworks::parse(&entries)
            .expect("valid networks")
            .expect("non-empty")
    }

    #[test]
    fn empty_list_is_no_rule() {
        assert!(BlockedNetworks::parse(&[]).expect("empty is valid").is_none());
    }

    #[test]
    fn invalid_entries_are_rejected() {
        for entry in [
            "",
            "10.0.0.0/33",
            "fd00::/129",
            "10.0.0.0/",
            "10.0.0.0/+8",
            "example.com",
            "10.0.0/8",
        ] {
            let result = BlockedNetworks::parse(&[entry.to_owned()]);
            assert!(
                matches!(result, Err(LiterLlmError::BadRequest { .. })),
                "{entry:?} should be rejected"
            );
        }
    }

    #[test]
    fn contains_follows_prefix_boundaries() {
        let networks = blocked(&["169.254.0.0/16", "10.1.2.0/23", "fd00::/8", "192.0.2.7"]);
        for (address, expected) in [
            ("169.254.169.254", true),
            ("169.255.0.0", false),
            ("10.1.2.1", true),
            ("10.1.3.255", true),
            ("10.1.4.0", false),
            ("192.0.2.7", true),
            ("192.0.2.8", false),
            ("fd12::1", true),
            ("fe00::1", false),
            ("::ffff:169.254.169.254", true),
            ("8.8.8.8", false),
        ] {
            let ip: IpAddr = address.parse().expect("IP");
            assert_eq!(networks.contains(ip), expected, "contains({address})");
        }
    }

    #[test]
    fn host_bits_are_cleared() {
        assert!(blocked(&["10.9.9.9/8"]).contains("10.0.0.1".parse().expect("IP")));
    }

    #[test]
    fn zero_prefix_blocks_its_family_only() {
        let networks = blocked(&["0.0.0.0/0"]);
        assert!(networks.contains("8.8.8.8".parse().expect("IP")));
        assert!(!networks.contains("2001:4860::1".parse().expect("IP")));
    }

    #[test]
    fn check_url_refuses_blocked_literal_hosts() {
        let networks = blocked(&["169.254.0.0/16", "fd00::/8"]);
        for url in ["http://169.254.169.254/latest/meta-data/", "https://[fd00::1]:8443/v1"] {
            assert!(
                matches!(networks.check_url(url), Err(LiterLlmError::OutboundForbidden { .. })),
                "{url} should be refused"
            );
        }
        for url in ["http://8.8.8.8/", "https://api.openai.com/v1", "not a url"] {
            assert!(networks.check_url(url).is_ok(), "{url} should pass");
        }
    }
}
