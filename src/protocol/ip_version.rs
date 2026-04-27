use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpVersion {
    Any,
    V4,
    V6,
}

impl IpVersion {
    pub fn matches_socket_addr(self, addr: &SocketAddr) -> bool {
        match self {
            Self::Any => true,
            Self::V4 => addr.is_ipv4(),
            Self::V6 => addr.is_ipv6(),
        }
    }

    pub fn matches_ip(self, addr: IpAddr) -> bool {
        match self {
            Self::Any => true,
            Self::V4 => addr.is_ipv4(),
            Self::V6 => addr.is_ipv6(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Any => "addresses",
            Self::V4 => "IPv4 addresses",
            Self::V6 => "IPv6 addresses",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Any => "IP",
            Self::V4 => "IPv4",
            Self::V6 => "IPv6",
        }
    }

    pub fn local_address(self) -> Option<IpAddr> {
        match self {
            Self::Any => None,
            Self::V4 => Some(Ipv4Addr::UNSPECIFIED.into()),
            Self::V6 => Some(Ipv6Addr::UNSPECIFIED.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_expected_address_family() {
        let ipv4 = "127.0.0.1:80".parse().unwrap();
        let ipv6 = "[::1]:80".parse().unwrap();

        assert!(IpVersion::Any.matches_socket_addr(&ipv4));
        assert!(IpVersion::Any.matches_socket_addr(&ipv6));
        assert!(IpVersion::V4.matches_socket_addr(&ipv4));
        assert!(!IpVersion::V4.matches_socket_addr(&ipv6));
        assert!(IpVersion::V6.matches_socket_addr(&ipv6));
        assert!(!IpVersion::V6.matches_socket_addr(&ipv4));
    }
}
