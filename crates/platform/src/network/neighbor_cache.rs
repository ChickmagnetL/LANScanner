use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;

use ssh_core::network::NetworkInterface;

use super::{
    Ipv4Subnet, NEIGHBOR_CANDIDATE_CACHE_PREFIX, NEIGHBOR_PRIMING_MAX_TARGETS,
    NEIGHBOR_PRIMING_TIMEOUT_MS, NEIGHBOR_PRIMING_UDP_PORT, NeighborCandidate,
    build_neighbor_candidates, collect_system_neighbor_rows, parse_ipv4_subnet,
    parse_neighbor_candidate_ip,
};

pub(super) async fn attempt_neighbor_priming(local_ip: Ipv4Addr, subnet: Ipv4Subnet) {
    let targets = build_neighbor_priming_targets(local_ip, subnet, NEIGHBOR_PRIMING_MAX_TARGETS);
    if targets.is_empty() {
        return;
    }

    let _ = tokio::time::timeout(
        Duration::from_millis(NEIGHBOR_PRIMING_TIMEOUT_MS),
        send_neighbor_priming_datagrams(local_ip, targets),
    )
    .await;
}

async fn send_neighbor_priming_datagrams(local_ip: Ipv4Addr, targets: Vec<Ipv4Addr>) {
    let Some(socket) = bind_neighbor_priming_socket(local_ip).await else {
        return;
    };

    for target in targets {
        let _ = socket
            .send_to(&[0u8], (target, NEIGHBOR_PRIMING_UDP_PORT))
            .await;
    }
}

async fn bind_neighbor_priming_socket(local_ip: Ipv4Addr) -> Option<tokio::net::UdpSocket> {
    if let Ok(socket) = tokio::net::UdpSocket::bind((local_ip, 0)).await {
        return Some(socket);
    }
    tokio::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .await
        .ok()
}

pub(super) fn build_neighbor_priming_targets(
    local_ip: Ipv4Addr,
    subnet: Ipv4Subnet,
    max_targets: usize,
) -> Vec<Ipv4Addr> {
    if max_targets == 0 {
        return Vec::new();
    }
    let Some((start, end)) = subnet.host_range() else {
        return Vec::new();
    };

    let local_raw = u32::from(local_ip);
    let host_count = u64::from(end.saturating_sub(start)) + 1;
    let mut selected = Vec::new();

    if host_count <= max_targets as u64 {
        for raw in start..=end {
            if raw == local_raw {
                continue;
            }
            selected.push(Ipv4Addr::from(raw));
        }
        return selected;
    }

    let mut seen = HashSet::new();
    for slot in 0..max_targets {
        let offset = (slot as u64 * host_count) / max_targets as u64;
        let raw = start.saturating_add(offset as u32).min(end);
        if raw == local_raw {
            continue;
        }
        if seen.insert(raw) {
            selected.push(Ipv4Addr::from(raw));
        }
    }

    if selected.len() < max_targets {
        for raw in start..=end {
            if selected.len() >= max_targets {
                break;
            }
            if raw == local_raw || !seen.insert(raw) {
                continue;
            }
            selected.push(Ipv4Addr::from(raw));
        }
    }

    selected.sort_unstable_by_key(|ip| ip.octets());
    selected
}

pub(super) fn neighbor_candidate_cache_path(local_ip: &str, subnet: &str) -> PathBuf {
    let sanitized_ip = sanitize_cache_component(local_ip);
    let sanitized_subnet = sanitize_cache_component(subnet);
    std::env::temp_dir().join(format!(
        "{NEIGHBOR_CANDIDATE_CACHE_PREFIX}-{sanitized_ip}-{sanitized_subnet}.txt"
    ))
}

fn sanitize_cache_component(value: &str) -> String {
    let mut sanitized = value
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    if sanitized.is_empty() {
        sanitized.push('_');
    }
    sanitized
}

pub(super) fn write_neighbor_candidate_cache(
    local_ip: &str,
    subnet: &str,
    candidates: &[NeighborCandidate],
) -> std::io::Result<()> {
    let path = neighbor_candidate_cache_path(local_ip, subnet);
    let mut content = String::new();
    for candidate in candidates {
        content.push_str(candidate.ip.trim());
        content.push('\n');
    }
    std::fs::write(path, content)
}

pub(super) fn remove_neighbor_candidate_cache(local_ip: &str, subnet: &str) {
    let path = neighbor_candidate_cache_path(local_ip, subnet);
    let _ = std::fs::remove_file(path);
}

pub(super) async fn prime_neighbor_candidate_cache(interfaces: &[NetworkInterface]) {
    if interfaces.is_empty() {
        return;
    }

    let rows = collect_system_neighbor_rows().await;
    for interface in interfaces {
        let Some(local_ip) = parse_neighbor_candidate_ip(&interface.local_ip) else {
            remove_neighbor_candidate_cache(&interface.local_ip, &interface.ip_range);
            continue;
        };
        let Some(subnet) = parse_ipv4_subnet(&interface.ip_range) else {
            remove_neighbor_candidate_cache(&interface.local_ip, &interface.ip_range);
            continue;
        };

        let candidates = build_neighbor_candidates(rows.clone(), local_ip, subnet);
        let _ =
            write_neighbor_candidate_cache(&interface.local_ip, &interface.ip_range, &candidates);
    }
}
