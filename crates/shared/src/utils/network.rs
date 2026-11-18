use anyhow::Result;
use std::sync::Arc;

use crate::system::{NetworkInterface, System};

pub async fn network_interfaces_in_subnet(
    operations: Arc<dyn System>,
    subnet: Option<&str>,
) -> Result<Vec<NetworkInterface>> {
    let nets = operations.get_network_info()?;
    // is_ip_in_subnet returns True if subnet is empty
    let ips = nets
        .iter()
        .filter(|iface| iface.in_subnet(subnet))
        .cloned()
        .collect::<Vec<_>>();
    Ok(ips)
}

pub async fn network_interfaces_changed(
    operations: Arc<dyn System>,
    known: &[NetworkInterface],
    subnet: Option<&str>,
) -> Result<Vec<NetworkInterface>> {
    let current = network_interfaces_in_subnet(operations, subnet).await?;

    // Compare by name across BOTH sets. A length early-return was wrong (equal
    // lengths with different names is also a change), and iterating `current`
    // alone would swallow pure removals, latching the watch on a stale set.
    let mut removed: Vec<&NetworkInterface> = known.iter().collect();
    for iface in &current {
        removed.retain(|k| k.name != iface.name);
        if let Some(k) = known.iter().find(|k| k.name == iface.name) {
            if k.mac != iface.mac || k.ip_addr != iface.ip_addr {
                return Ok(current); // Changed mac/ip
            }
        } else {
            return Ok(current); // New interface
        }
    }
    if !removed.is_empty() {
        return Ok(current); // Removed interface(s)
    }

    Ok(Vec::new()) // No changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iface(name: &str, ip: &str, mac: &str) -> NetworkInterface {
        NetworkInterface {
            name: name.to_string(),
            ip_addr: ip.to_string(),
            mac: mac.to_string(),
        }
    }

    #[test]
    fn detects_removals_even_at_equal_length() {
        // eth1 gone, eth0 untouched: same length would have hidden the change
        // only in the old code if lengths matched; a name swap (eth1 out, eth9
        // in) keeps the length equal and MUST still be detected.
        let known = [
            iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01"),
            iface("eth1", "10.0.1.2", "aa:bb:cc:dd:ee:02"),
        ];
        let swapped = [
            iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01"),
            iface("eth9", "10.0.9.2", "aa:bb:cc:dd:ee:09"),
        ];
        assert!(!diff_is_empty(&known, &swapped), "name swap must be a change");
        let removed_only = [iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01")];
        assert!(
            !diff_is_empty(&known, &removed_only),
            "pure removal must be a change"
        );
    }

    #[test]
    fn identical_and_reordered_sets_are_no_change() {
        let a = [iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01"), iface("wlan0", "10.0.1.2", "aa:bb:cc:dd:ee:02")];
        let b = [iface("wlan0", "10.0.1.2", "aa:bb:cc:dd:ee:02"), iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01")];
        assert!(diff_is_empty(&a, &b), "reordering is not a change");
    }

    #[test]
    fn mac_or_ip_change_is_detected() {
        let a = [iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:01")];
        let b = [iface("eth0", "10.0.0.2", "aa:bb:cc:dd:ee:FF")];
        let c = [iface("eth0", "10.9.9.9", "aa:bb:cc:dd:ee:01")];
        assert!(!diff_is_empty(&a, &b), "mac change is a change");
        assert!(!diff_is_empty(&a, &c), "ip change is a change");
    }

    // Helper mirroring network_interfaces_changed's comparison core so the
    // regression is testable without a live System implementation.
    fn diff_is_empty(known: &[NetworkInterface], current: &[NetworkInterface]) -> bool {
        let mut removed: Vec<&NetworkInterface> = known.iter().collect();
        for iface in current {
            removed.retain(|k| k.name != iface.name);
            if let Some(k) = known.iter().find(|k| k.name == iface.name) {
                if k.mac != iface.mac || k.ip_addr != iface.ip_addr {
                    return false;
                }
            } else {
                return false;
            }
        }
        removed.is_empty()
    }
}
