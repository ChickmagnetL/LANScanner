use std::net::Ipv4Addr;

use ssh_core::scanner::NeighborEvidence;
#[cfg(any(
    target_os = "macos",
    all(unix, not(target_os = "windows"), not(target_os = "macos"))
))]
use tokio::process::Command;

use super::NeighborEvidenceRow;

#[cfg(target_os = "windows")]
pub(super) async fn collect_windows_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    let command = [
        "Get-NetNeighbor -AddressFamily IPv4 -ErrorAction SilentlyContinue |",
        "ForEach-Object { \"$($_.IPAddress)|$($_.LinkLayerAddress)|$($_.State)\" }",
    ]
    .join(" ");

    let Some(stdout) = super::run_windows_powershell(&command).await else {
        return Vec::new();
    };

    stdout
        .lines()
        .filter_map(parse_windows_neighbor_row)
        .collect()
}

#[cfg(any(target_os = "windows", test))]
pub(super) fn parse_windows_neighbor_row(line: &str) -> Option<NeighborEvidenceRow> {
    let mut parts = line.split('|');
    let ip = parts.next()?.trim();
    let mac = parts.next().unwrap_or_default().trim();
    let state = parts.next().unwrap_or_default().trim();

    if !is_valid_neighbor_ip(ip) || is_unusable_neighbor_state(state) {
        return None;
    }

    let mac_address = normalize_neighbor_mac(mac)?;
    Some(NeighborEvidenceRow {
        ip: ip.to_owned(),
        evidence: NeighborEvidence::new(Some(mac_address), None, None),
    })
}

#[cfg(all(unix, not(target_os = "windows"), not(target_os = "macos")))]
pub(super) async fn collect_unix_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    let ip_rows =
        collect_neighbor_rows_with_command("ip", &["neigh", "show"], parse_unix_neighbor_row).await;
    if !ip_rows.is_empty() {
        return ip_rows;
    }

    collect_neighbor_rows_with_command("arp", &["-an"], parse_arp_neighbor_row).await
}

#[cfg(any(test, all(unix, not(target_os = "windows"), not(target_os = "macos"))))]
pub(super) fn parse_unix_neighbor_row(line: &str) -> Option<NeighborEvidenceRow> {
    let mut parts = line.split_whitespace();
    let ip = parts.next()?.trim();

    if !is_valid_neighbor_ip(ip) {
        return None;
    }

    let mut mac_address = None;
    let mut is_failed = false;

    while let Some(token) = parts.next() {
        if token.eq_ignore_ascii_case("lladdr") {
            mac_address = parts.next().and_then(normalize_neighbor_mac);
            continue;
        }

        if token.eq_ignore_ascii_case("failed")
            || token.eq_ignore_ascii_case("incomplete")
            || token.eq_ignore_ascii_case("unreachable")
        {
            is_failed = true;
        }
    }

    if is_failed {
        return None;
    }

    Some(NeighborEvidenceRow {
        ip: ip.to_owned(),
        evidence: NeighborEvidence::new(mac_address, None, None),
    })
    .filter(|row| row.evidence.mac_address.is_some())
}

#[cfg(target_os = "macos")]
pub(super) async fn collect_macos_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    let rows =
        collect_neighbor_rows_with_command("arp", &["-a", "-n"], parse_arp_neighbor_row).await;
    if !rows.is_empty() {
        return rows;
    }

    collect_neighbor_rows_with_command("arp", &["-an"], parse_arp_neighbor_row).await
}

#[cfg(any(target_os = "macos", test))]
pub(super) fn parse_macos_arp_row(line: &str) -> Option<NeighborEvidenceRow> {
    parse_arp_neighbor_row(line)
}

#[cfg(any(
    test,
    target_os = "macos",
    all(unix, not(target_os = "windows"), not(target_os = "macos"))
))]
fn parse_arp_neighbor_row(line: &str) -> Option<NeighborEvidenceRow> {
    let left = line.find('(')?;
    let right = line[left + 1..].find(')')? + left + 1;
    let ip = line[left + 1..right].trim();
    if !is_valid_neighbor_ip(ip) {
        return None;
    }

    let host = line[..left].trim();
    let at_pos = line.find(" at ")?;
    let mac = line[at_pos + 4..]
        .split_whitespace()
        .next()
        .unwrap_or_default();
    let mac_address = normalize_neighbor_mac(mac)?;

    let hostname = trim_neighbor_hostname(host).map(str::to_owned);
    let mdns_name = hostname
        .as_deref()
        .filter(|name| name.ends_with(".local"))
        .map(str::to_owned);

    Some(NeighborEvidenceRow {
        ip: ip.to_owned(),
        evidence: NeighborEvidence::new(Some(mac_address), hostname, mdns_name),
    })
}

#[cfg(any(
    target_os = "macos",
    all(unix, not(target_os = "windows"), not(target_os = "macos"))
))]
async fn collect_neighbor_rows_with_command(
    program: &str,
    args: &[&str],
    parser: fn(&str) -> Option<NeighborEvidenceRow>,
) -> Vec<NeighborEvidenceRow> {
    let Ok(output) = Command::new(program).args(args).output().await else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parser)
        .collect()
}

#[cfg(any(target_os = "windows", test))]
fn is_unusable_neighbor_state(state: &str) -> bool {
    let state = state.trim();
    if state.is_empty() {
        return false;
    }
    let lower = state.to_ascii_lowercase();
    lower.contains("unreachable")
        || lower.contains("incomplete")
        || lower.contains("failed")
        || lower.contains("invalid")
}

fn is_valid_neighbor_ip(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>()
        .ok()
        .is_some_and(|addr| !addr.is_loopback() && !addr.is_link_local() && !addr.is_unspecified())
}

fn normalize_neighbor_mac(raw: &str) -> Option<String> {
    let hex = raw
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();

    if hex.len() != 12 {
        return None;
    }

    let mut normalized = String::with_capacity(17);
    for index in 0..6 {
        if index > 0 {
            normalized.push(':');
        }
        let offset = index * 2;
        normalized.push_str(&hex[offset..offset + 2]);
    }

    Some(normalized.to_ascii_uppercase())
}

#[cfg(any(
    test,
    target_os = "macos",
    all(unix, not(target_os = "windows"), not(target_os = "macos"))
))]
fn trim_neighbor_hostname(host: &str) -> Option<&str> {
    let host = host.trim().trim_matches('?').trim_matches('"').trim();
    if host.is_empty() || host.chars().any(char::is_control) {
        return None;
    }
    Some(host)
}
