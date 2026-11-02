//! A per-client list of networks that the client never connects to.

use std::net::IpAddr;

use ipnet::IpNet;

use crate::error::LiterLlmError;
use crate::provider::embedded_ip::{embedded_ipv4, is_isatap};

/// The blocked networks of one client, parsed from CIDR strings.
///
/// An address is refused when it, or the IPv4 address it carries, is in a listed
/// network. Every ISATAP address is refused, whatever the list holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRule {
    networks: Vec<IpNet>,
}

impl NetworkRule {
    /// Parse a list of CIDR strings such as `"169.254.0.0/16"` or `"64:ff9b::/96"`.
    ///
    /// # Errors
    /// Returns [`LiterLlmError::BadRequest`] naming the first entry that is not a CIDR network.
    pub fn parse(entries: &[String]) -> Result<Self, LiterLlmError> {
        let networks = entries
            .iter()
            .map(|entry| {
                entry.trim().parse::<IpNet>().map_err(|_| LiterLlmError::BadRequest {
                    message: format!("blocked_networks entry {entry:?} is not a CIDR network"),
                    status: 400,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { networks })
    }

    /// True when the client must not connect to `ip`.
    #[must_use]
    pub fn refuses(&self, ip: IpAddr) -> bool {
        let contains = |candidate: IpAddr| self.networks.iter().any(|network| network.contains(&candidate));
        match ip {
            IpAddr::V4(_) => contains(ip),
            IpAddr::V6(v6) => {
                is_isatap(v6) || contains(ip) || embedded_ipv4(v6).is_some_and(|v4| contains(IpAddr::V4(v4)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(entries: &[&str]) -> NetworkRule {
        NetworkRule::parse(&entries.iter().map(|e| (*e).to_owned()).collect::<Vec<_>>()).expect("valid list")
    }

    #[test]
    fn embedded_forms_carry_their_ipv4() {
        let blocked = rule(&["169.254.0.0/16"]);
        for form in [
            "169.254.169.254",
            "::ffff:169.254.169.254",
            "2002:a9fe:a9fe::",
            "::ffff:0:169.254.169.254",
            "::169.254.169.254",
            "64:ff9b::169.254.169.254",
            "2001:0:4136:e378:8000:63bf:5601:5601",
        ] {
            assert!(
                blocked.refuses(form.parse().expect("address")),
                "{form} must be refused"
            );
        }
        for form in [
            "8.8.8.8",
            "::ffff:8.8.8.8",
            "2002:808:808::",
            "::1",
            "::",
            "2001:db8::1",
        ] {
            assert!(
                !blocked.refuses(form.parse().expect("address")),
                "{form} must be allowed"
            );
        }
    }
}
