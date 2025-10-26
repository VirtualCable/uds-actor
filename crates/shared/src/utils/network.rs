use std::sync::Arc;
use anyhow::Result;

use crate::operations::{NetworkInterface, Operations};

pub async fn network_interfaces_in_subnet(
    operations: Arc<dyn Operations>,
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
    operations: Arc<dyn Operations>,
    known: &[NetworkInterface],
    subnet: Option<&str>,
) -> Result<Vec<NetworkInterface>> {
    let current = network_interfaces_in_subnet(operations, subnet).await?;

    if known.len() != current.len() {
        return Ok(Vec::new()); // null change
    }

    // The order can be not the same, compare 1 by 1
    for iface in &current {
        if let Some(k) = known.iter().find(|k| k.name == iface.name) {
            if k.mac != iface.mac || k.ip_addr != iface.ip_addr {
                return Ok(current); // If any difference, return all
            }
        } else {
            return Ok(current); // New interface
        }
    }

    Ok(Vec::new()) // No changes
}
