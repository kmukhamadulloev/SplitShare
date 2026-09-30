//! Socket-peer classification. Forwarding headers are never authorization inputs.
use std::{collections::HashSet, net::IpAddr};

#[derive(Clone, Default)]
pub struct LocalAddresses(HashSet<IpAddr>, Vec<(String, IpAddr)>);
impl LocalAddresses {
    pub fn discover() -> std::io::Result<Self> {
        let interfaces = if_addrs::get_if_addrs()?
            .into_iter()
            .map(|interface| (interface.name.clone(), interface.ip()))
            .collect::<Vec<_>>();
        Ok(Self(
            interfaces.iter().map(|(_, ip)| *ip).collect(),
            interfaces,
        ))
    }
    pub fn candidates(&self, bind: std::net::SocketAddr) -> Vec<AddressCandidate> {
        candidates(&self.1, bind)
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
        let addresses = LocalAddresses(["192.168.1.2".parse().unwrap()].into(), vec![]);
        for local in ["127.0.0.1", "::1", "::ffff:127.0.0.1", "192.168.1.2"] {
            assert!(addresses.is_local(local.parse().unwrap()));
        }
        for remote in ["192.168.1.3", "10.0.0.1", "8.8.8.8"] {
            assert!(!addresses.is_local(remote.parse().unwrap()));
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct AddressCandidate {
    pub interface: String,
    pub address: String,
    pub base_url: String,
    pub kind: &'static str,
}
pub fn candidates(
    interfaces: &[(String, IpAddr)],
    bind: std::net::SocketAddr,
) -> Vec<AddressCandidate> {
    let mut result = Vec::new();
    for (name, address) in interfaces {
        let IpAddr::V4(ip) = address else {
            continue;
        };
        if ip.is_unspecified()
            || ip.is_multicast()
            || (!bind.ip().is_unspecified() && bind.ip() != *address)
        {
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let kind = if ip.is_loopback() {
            "loopback"
        } else if ["docker", "veth", "virbr", "br-", "vmnet"]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        {
            "virtual"
        } else if ["wg", "tun", "tap", "tailscale", "utun"]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
        {
            "vpn"
        } else if ip.is_private() {
            "private"
        } else if ip.is_link_local() {
            "link_local"
        } else {
            "other"
        };
        result.push(AddressCandidate {
            interface: name.clone(),
            address: ip.to_string(),
            base_url: format!("http://{ip}:{}", bind.port()),
            kind,
        });
    }
    result.sort_by_key(|entry| {
        (
            match entry.kind {
                "private" => 0,
                "vpn" => 1,
                "link_local" => 2,
                "other" => 3,
                "virtual" => 4,
                _ => 5,
            },
            entry.address.clone(),
            entry.interface.clone(),
        )
    });
    result.dedup_by(|a, b| a.base_url == b.base_url);
    result
}

#[cfg(test)]
mod candidate_tests {
    use super::*;
    #[test]
    fn preserves_multiple_adapters_and_obeys_binding() {
        let interfaces = [
            ("docker0", "172.17.0.1"),
            ("wg0", "10.0.0.2"),
            ("wlan0", "192.168.1.20"),
            ("eth0", "192.168.2.20"),
            ("lo", "127.0.0.1"),
            ("eth0", "fe80::1"),
        ]
        .map(|(name, ip)| (name.into(), ip.parse().unwrap()));
        let all = candidates(&interfaces, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(all.len(), 5);
        assert_eq!(all.iter().filter(|c| c.kind == "private").count(), 2);
        assert_eq!(all[2].kind, "vpn");
        assert!(
            all.iter()
                .all(|c| c.base_url.starts_with("http://") && c.base_url.ends_with(":8080"))
        );
        let bound = candidates(&interfaces, "192.168.2.20:1234".parse().unwrap());
        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0].base_url, "http://192.168.2.20:1234");
        assert_eq!(
            candidates(&interfaces, "127.0.0.1:8080".parse().unwrap())[0].kind,
            "loopback"
        );
    }
}
