//! Socket-peer classification. Forwarding headers are never authorization inputs.
use std::{collections::HashSet, net::IpAddr};

#[derive(Clone, Default)]
pub struct LocalAddresses(HashSet<IpAddr>);
impl LocalAddresses {
    pub fn discover() -> std::io::Result<Self> {
        Ok(Self(
            if_addrs::get_if_addrs()?
                .into_iter()
                .map(|interface| interface.ip())
                .collect(),
        ))
    }
    pub fn authorities(&self, port: u16) -> Vec<String> {
        let mut result = vec![format!("localhost:{port}"), format!("127.0.0.1:{port}")];
        result.extend(
            self.0
                .iter()
                .filter(|ip| ip.is_ipv4())
                .map(|ip| format!("{ip}:{port}")),
        );
        result
    }
    pub fn is_local(&self, peer: IpAddr) -> bool {
        let peer = match peer {
            IpAddr::V6(ip) => ip.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(peer),
            _ => peer,
        };
        peer.is_loopback() || self.0.contains(&peer)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_only_loopback_or_actual_interface() {
        let addresses = LocalAddresses(["192.168.1.2".parse().unwrap()].into());
        for local in ["127.0.0.1", "::1", "::ffff:127.0.0.1", "192.168.1.2"] {
            assert!(addresses.is_local(local.parse().unwrap()));
        }
        for remote in ["192.168.1.3", "10.0.0.1", "8.8.8.8"] {
            assert!(!addresses.is_local(remote.parse().unwrap()));
        }
    }
}
