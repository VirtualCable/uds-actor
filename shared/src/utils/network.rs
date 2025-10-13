use ipnetwork::IpNetwork;
use std::net::IpAddr;
use std::str::FromStr;

pub fn is_ip_in_subnet(ip: &str, subnet: &str) -> bool {
    // If no IP provided, return false
    if ip.trim().is_empty() {
        return false;
    }

    // If no subnet provided, return true (meaning any IP is allowed)
    if subnet.trim().is_empty() {
        return true;
    }

    // Parse the IP and subnet
    let ip = match IpAddr::from_str(ip) {
        Ok(addr) => addr,
        Err(_) => return false,
    };

    let subnet = match IpNetwork::from_str(subnet) {
        Ok(net) => net,
        Err(_) => return false,
    };

    subnet.contains(ip)
}
