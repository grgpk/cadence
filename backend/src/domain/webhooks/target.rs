//! Where a webhook may go: a public HTTPS address only.
//!
//! A host chooses the URL and Cadence's server calls it, so it must never
//! reach this server, its network, or a cloud metadata address. The name is
//! checked when it is saved, and the address it resolves to is checked again,
//! and used, on every delivery.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use reqwest::Url;

const URL_MAX: usize = 500;
/// Names that only ever mean a private network.
const PRIVATE_SUFFIXES: [&str; 4] = [".localhost", ".local", ".internal", ".lan"];

/// A URL a webhook may be saved with.
pub fn check_url(raw: &str) -> Result<Url, &'static str> {
    let url = Url::parse(raw.trim()).map_err(|_| "the URL is not valid")?;
    if raw.len() > URL_MAX {
        return Err("the URL is longer than 500 characters");
    }
    if url.scheme() != "https" {
        return Err("the URL must start with https://");
    }
    let host = url.host_str().ok_or("the URL has no host")?;
    let public = bare(host).parse::<IpAddr>().map_or_else(
        |_| {
            let name = host.to_ascii_lowercase();
            name != "localhost" && !PRIVATE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        },
        is_public,
    );
    if !public {
        return Err("the URL must be a public address");
    }
    Ok(url)
}

/// The one address a delivery goes to. Every address the name resolves to
/// must be public, so a name cannot point at this server between checks.
pub async fn resolve(url: &Url) -> Result<SocketAddr, String> {
    let host = bare(url.host_str().ok_or("the URL has no host")?);
    let port = url.port_or_known_default().ok_or("the URL has no port")?;
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("{host} did not resolve: {error}"))?
        .collect();
    match addresses.first() {
        None => Err(format!("{host} did not resolve")),
        Some(_) if addresses.iter().any(|address| !is_public(address.ip())) => {
            Err(format!("{host} resolves to a private address"))
        }
        Some(address) => Ok(*address),
    }
}

/// `host_str` keeps an IPv6 address in brackets; parsers want it bare.
fn bare(host: &str) -> &str {
    host.trim_start_matches('[').trim_end_matches(']')
}

/// An address on the open internet: not loopback, private, link-local,
/// shared, documentation, multicast or unspecified.
pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_v4(ip),
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map_or_else(|| is_public_v6(ip), is_public_v4),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || a == 0
        // Carrier-grade NAT, 100.64.0.0/10.
        || (a == 100 && (64..128).contains(&b))
        // Benchmarking, 198.18.0.0/15.
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240)
}

const fn is_public_v6(ip: Ipv6Addr) -> bool {
    let [first, second, ..] = ip.segments();
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // Unique local, fc00::/7.
        || (first & 0xfe00) == 0xfc00
        // Link-local, fe80::/10.
        || (first & 0xffc0) == 0xfe80
        // Documentation, 2001:db8::/32.
        || (first == 0x2001 && second == 0x0db8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_https_urls_are_saved() {
        assert!(check_url("https://hooks.example.com/cadence").is_ok());
        assert!(check_url("https://93.184.216.34/hook").is_ok());
        for bad in [
            "http://hooks.example.com/cadence",
            "https://localhost/hook",
            "https://printer.local/hook",
            "https://metadata.google.internal/hook",
            "https://127.0.0.1/hook",
            "https://10.0.0.5/hook",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/hook",
            "https://[fd00::1]/hook",
            // 127.0.0.1 written as one number.
            "https://2130706433/hook",
            "not a url",
        ] {
            assert!(check_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn private_and_special_addresses_are_not_public() {
        for ip in [
            "192.168.1.1",
            "172.16.0.1",
            "100.64.0.1",
            "0.0.0.0",
            "198.18.0.1",
            "::ffff:10.0.0.1",
        ] {
            let ip: IpAddr = ip.parse().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert!(!is_public(ip), "{ip}");
        }
        for ip in ["8.8.8.8", "2606:4700:4700::1111"] {
            let ip: IpAddr = ip.parse().unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
            assert!(is_public(ip), "{ip}");
        }
    }
}
