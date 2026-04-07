use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::{Arc, Once};

use ssh_core::network::{self, InterfaceType, NetworkDetector, NetworkInterface};
use ssh_core::scanner::NeighborEvidence;
use tokio::process::Command;

#[cfg(target_os = "windows")]
use crate::process;
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};

mod neighbor_cache;
mod neighbor_rows;

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WifiAvailability {
    Connected,
    ExplicitlyUnavailable,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WifiObservation {
    availability: WifiAvailability,
    ssid: Option<String>,
}

#[cfg(any(target_os = "windows", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct WindowsRefreshInputs {
    ipv4_snapshot: String,
    wifi_snapshot: String,
    profile_snapshot: String,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsRefreshCacheEntry {
    inputs: WindowsRefreshInputs,
    interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NeighborEvidenceRow {
    ip: String,
    evidence: NeighborEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighborCandidate {
    pub ip: String,
    pub evidence: NeighborEvidence,
}

const NEIGHBOR_CANDIDATE_CACHE_PREFIX: &str = "sshscanner-neighbor-candidates";
const NEIGHBOR_PRIMING_MAX_TARGETS: usize = 256;
const NEIGHBOR_PRIMING_UDP_PORT: u16 = 33434;
const NEIGHBOR_PRIMING_TIMEOUT_MS: u64 = 800;

pub fn ensure_registered() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        network::register_detector(Arc::new(SystemNetworkDetector));
    });
}

pub async fn collect_neighbor_evidence(
    discovered_ips: &[String],
) -> HashMap<String, NeighborEvidence> {
    if discovered_ips.is_empty() {
        return HashMap::new();
    }

    let discovered: HashSet<_> = discovered_ips.iter().map(String::as_str).collect();
    let mut evidence_by_ip = HashMap::new();

    for row in collect_system_neighbor_rows().await {
        if !discovered.contains(row.ip.as_str()) {
            continue;
        }

        evidence_by_ip
            .entry(row.ip)
            .and_modify(|current| merge_neighbor_evidence(current, row.evidence.clone()))
            .or_insert(row.evidence);
    }

    evidence_by_ip
}

pub async fn discover_online_neighbor_candidates(
    local_ip: &str,
    subnet: &str,
) -> Vec<NeighborCandidate> {
    let Some(local_ip_addr) = parse_neighbor_candidate_ip(local_ip) else {
        remove_neighbor_candidate_cache(local_ip, subnet);
        return Vec::new();
    };
    let Some(subnet_cidr) = parse_ipv4_subnet(subnet) else {
        remove_neighbor_candidate_cache(local_ip, subnet);
        return Vec::new();
    };

    let rows = collect_system_neighbor_rows().await;
    let candidates = build_neighbor_candidates(rows, local_ip_addr, subnet_cidr);
    attempt_neighbor_priming(local_ip_addr, subnet_cidr).await;
    let candidates = refresh_neighbor_candidates_after_priming(
        candidates,
        collect_system_neighbor_rows().await,
        local_ip_addr,
        subnet_cidr,
    );
    let _ = write_neighbor_candidate_cache(local_ip, subnet, &candidates);
    candidates
}

pub async fn discover_online_neighbor_dataset(
    local_ip: &str,
    subnet: &str,
) -> (Vec<String>, HashMap<String, NeighborEvidence>) {
    let candidates = discover_online_neighbor_candidates(local_ip, subnet).await;
    split_neighbor_candidates(candidates)
}

pub fn split_neighbor_candidates(
    candidates: Vec<NeighborCandidate>,
) -> (Vec<String>, HashMap<String, NeighborEvidence>) {
    if candidates.is_empty() {
        return (Vec::new(), HashMap::new());
    }

    let mut evidence_by_ip = HashMap::with_capacity(candidates.len());
    for NeighborCandidate { ip, evidence } in candidates {
        if let Some(current) = evidence_by_ip.get_mut(ip.as_str()) {
            merge_neighbor_evidence(current, evidence);
        } else {
            evidence_by_ip.insert(ip, evidence);
        }
    }

    let mut candidate_ips = evidence_by_ip.keys().cloned().collect::<Vec<_>>();
    candidate_ips.sort_by(|left, right| compare_neighbor_candidate_ip(left, right));
    (candidate_ips, evidence_by_ip)
}

struct SystemNetworkDetector;

impl NetworkDetector for SystemNetworkDetector {
    fn detect_interfaces(
        &self,
    ) -> Pin<Box<dyn Future<Output = Vec<NetworkInterface>> + Send + '_>> {
        Box::pin(detect_system_interfaces())
    }
}

async fn detect_system_interfaces() -> Vec<NetworkInterface> {
    let interfaces = {
        #[cfg(target_os = "windows")]
        {
            detect_windows_interfaces().await
        }

        #[cfg(not(target_os = "windows"))]
        {
            detect_unix_like_interfaces().await
        }
    };
    prime_neighbor_candidate_cache(&interfaces).await;
    interfaces
}

async fn collect_system_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    #[cfg(target_os = "windows")]
    {
        return collect_windows_neighbor_rows().await;
    }

    #[cfg(target_os = "macos")]
    {
        return collect_macos_neighbor_rows().await;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return collect_unix_neighbor_rows().await;
    }

    #[allow(unreachable_code)]
    Vec::new()
}

#[cfg(target_os = "windows")]
async fn detect_windows_interfaces() -> Vec<NetworkInterface> {
    let (ipv4_rows, wifi_observations, profiles) = tokio::join!(
        collect_windows_ipv4_rows(),
        collect_windows_wifi_observations(),
        collect_windows_connection_profiles()
    );

    let Some(ipv4_rows) = ipv4_rows else {
        return fallback_default_interface();
    };

    let inputs = build_windows_refresh_inputs(&ipv4_rows, &wifi_observations, &profiles);
    if let Some(cached) = try_windows_refresh_fast_path(&inputs) {
        return cached;
    }

    let interfaces = build_windows_interfaces_from_rows(&ipv4_rows, &wifi_observations, &profiles);

    if interfaces.is_empty() {
        fallback_default_interface()
    } else {
        remember_windows_refresh_snapshot(inputs, interfaces.clone());
        interfaces
    }
}

#[cfg(target_os = "windows")]
fn windows_refresh_cache() -> &'static Mutex<Option<WindowsRefreshCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<WindowsRefreshCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

#[cfg(target_os = "windows")]
fn try_windows_refresh_fast_path(inputs: &WindowsRefreshInputs) -> Option<Vec<NetworkInterface>> {
    let guard = windows_refresh_cache().lock().ok()?;
    guard
        .as_ref()
        .and_then(|cached| (cached.inputs == *inputs).then(|| cached.interfaces.clone()))
}

#[cfg(target_os = "windows")]
fn remember_windows_refresh_snapshot(
    inputs: WindowsRefreshInputs,
    interfaces: Vec<NetworkInterface>,
) {
    if let Ok(mut guard) = windows_refresh_cache().lock() {
        *guard = Some(WindowsRefreshCacheEntry { inputs, interfaces });
    }
}

#[cfg(target_os = "windows")]
async fn collect_windows_ipv4_rows() -> Option<Vec<String>> {
    let command = [
        "$items = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |",
        "Where-Object { $_.IPAddress -and $_.InterfaceAlias -and $_.IPAddress -ne '127.0.0.1' } |",
        "ForEach-Object { \"$($_.InterfaceAlias)|$($_.IPAddress)|$($_.PrefixLength)\" };",
        "$items",
    ]
    .join(" ");

    let stdout = run_windows_powershell(&command).await?;
    Some(
        stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

#[cfg(target_os = "windows")]
async fn collect_windows_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    neighbor_rows::collect_windows_neighbor_rows().await
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_neighbor_row(line: &str) -> Option<NeighborEvidenceRow> {
    neighbor_rows::parse_windows_neighbor_row(line)
}

#[cfg(any(target_os = "windows", test))]
fn canonical_windows_ipv4_snapshot(rows: &[String]) -> String {
    let mut normalized = rows
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.join("\n")
}

#[cfg(any(target_os = "windows", test))]
fn canonical_windows_wifi_snapshot(wifi_observations: &HashMap<String, WifiObservation>) -> String {
    let mut normalized = wifi_observations
        .iter()
        .map(|(alias, observation)| {
            let availability = match observation.availability {
                WifiAvailability::Connected => "connected",
                WifiAvailability::ExplicitlyUnavailable => "unavailable",
            };
            format!(
                "{}|{}|{}",
                alias.trim(),
                availability,
                observation.ssid.as_deref().unwrap_or_default().trim()
            )
        })
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.join("\n")
}

#[cfg(any(target_os = "windows", test))]
fn canonical_windows_profile_snapshot(profiles: &HashMap<String, String>) -> String {
    let mut normalized = profiles
        .iter()
        .map(|(alias, profile)| format!("{}|{}", alias.trim(), profile.trim()))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.join("\n")
}

#[cfg(any(target_os = "windows", test))]
fn build_windows_refresh_inputs(
    rows: &[String],
    wifi_observations: &HashMap<String, WifiObservation>,
    profiles: &HashMap<String, String>,
) -> WindowsRefreshInputs {
    WindowsRefreshInputs {
        ipv4_snapshot: canonical_windows_ipv4_snapshot(rows),
        wifi_snapshot: canonical_windows_wifi_snapshot(wifi_observations),
        profile_snapshot: canonical_windows_profile_snapshot(profiles),
    }
}

#[cfg(any(target_os = "windows", test))]
fn parse_windows_ipv4_row(line: &str) -> Option<(&str, &str, u8)> {
    let mut parts = line.split('|');
    let alias = parts.next()?.trim();
    let ip = parts.next()?.trim();
    let prefix = parts.next()?.trim().parse::<u8>().ok()?;

    (!alias.is_empty() && !ip.is_empty()).then_some((alias, ip, prefix))
}

#[cfg(any(target_os = "windows", test))]
fn build_windows_interfaces_from_rows(
    rows: &[String],
    wifi_observations: &HashMap<String, WifiObservation>,
    profiles: &HashMap<String, String>,
) -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();

    for line in rows {
        let Some((alias, ip, prefix)) = parse_windows_ipv4_row(line) else {
            continue;
        };

        let iface_type = classify_interface(alias);
        let preferred_name = match iface_type {
            // 只有“明确不可用”的 Wi-Fi 才过滤；若 SSID 解析失败但接口仍可能可用，则保留并回退到 profile/alias。
            InterfaceType::Wifi => match resolve_wifi_candidate(
                wifi_observations.get(alias),
                profiles.get(alias).map(String::as_str),
            ) {
                WifiCandidate::Exclude => continue,
                WifiCandidate::Keep { preferred_name } => preferred_name,
            },
            _ => profiles.get(alias).map(String::as_str),
        };

        if let Some(interface) =
            build_interface(alias, alias, ip, prefix, iface_type, preferred_name)
        {
            interfaces.push(interface);
        }
    }

    dedupe_interfaces(interfaces)
}

#[cfg(target_os = "windows")]
async fn collect_windows_wifi_observations() -> HashMap<String, WifiObservation> {
    let Some(stdout) = run_windows_powershell("netsh wlan show interfaces").await else {
        return HashMap::new();
    };
    let mut current_name: Option<String> = None;
    let mut current_state: Option<String> = None;
    let mut current_ssid: Option<String> = None;
    let mut entries = HashMap::new();

    let flush_entry = |entries: &mut HashMap<String, WifiObservation>,
                       current_name: &mut Option<String>,
                       current_state: &mut Option<String>,
                       current_ssid: &mut Option<String>| {
        if let Some(name) = current_name.take() {
            let state = current_state.take().unwrap_or_default();
            let ssid = current_ssid.take().unwrap_or_default();

            let Some(availability) = classify_wifi_availability(&state) else {
                return;
            };

            let ssid = if availability == WifiAvailability::Connected && is_valid_ssid(&ssid) {
                Some(normalize_wifi_name(&ssid))
            } else {
                None
            };

            if !name.trim().is_empty() {
                entries.insert(name, WifiObservation { availability, ssid });
            }
        } else {
            current_state.take();
            current_ssid.take();
        }
    };

    for line in stdout.lines().map(str::trim) {
        if line.is_empty() {
            flush_entry(
                &mut entries,
                &mut current_name,
                &mut current_state,
                &mut current_ssid,
            );
            continue;
        }

        let Some((key, value)) = split_windows_kv(line) else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();
        let key_lower = key.to_ascii_lowercase();

        if is_name_key(key, &key_lower) {
            if current_name.is_some() {
                flush_entry(
                    &mut entries,
                    &mut current_name,
                    &mut current_state,
                    &mut current_ssid,
                );
            }
            current_name = Some(value.to_owned());
        } else if is_state_key(key, &key_lower) {
            current_state = Some(value.to_owned());
        } else if is_ssid_key(&key_lower) && is_valid_ssid(value) {
            current_ssid = Some(value.to_owned());
        }
    }

    flush_entry(
        &mut entries,
        &mut current_name,
        &mut current_state,
        &mut current_ssid,
    );

    entries
}

#[cfg(target_os = "windows")]
fn split_windows_kv(line: &str) -> Option<(&str, &str)> {
    line.split_once(':')
        .or_else(|| line.split_once('：'))
        .map(|(key, value)| (key.trim(), value.trim()))
}

#[cfg(target_os = "windows")]
fn is_name_key(raw_key: &str, key_lower: &str) -> bool {
    key_lower == "name" || raw_key == "名称"
}

#[cfg(target_os = "windows")]
fn is_state_key(raw_key: &str, key_lower: &str) -> bool {
    key_lower == "state" || raw_key == "状态"
}

#[cfg(target_os = "windows")]
fn is_ssid_key(key_lower: &str) -> bool {
    key_lower.starts_with("ssid") && !key_lower.starts_with("bssid")
}

#[cfg(target_os = "windows")]
fn is_connected_state(state: &str) -> bool {
    let state_lower = state.to_ascii_lowercase();
    state.contains("已连接")
        || (state_lower.contains("connected") && !state_lower.contains("disconnected"))
}

#[cfg(target_os = "windows")]
fn classify_wifi_availability(state: &str) -> Option<WifiAvailability> {
    let state = state.trim();
    if state.is_empty() {
        return None;
    }

    if is_connected_state(state) {
        Some(WifiAvailability::Connected)
    } else if is_explicitly_unavailable_state(state) {
        Some(WifiAvailability::ExplicitlyUnavailable)
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn is_explicitly_unavailable_state(state: &str) -> bool {
    let state_lower = state.to_ascii_lowercase();
    state.contains("未连接")
        || state.contains("断开")
        || state.contains("不可用")
        || state_lower.contains("disconnected")
        || state_lower.contains("not connected")
        || state_lower.contains("not available")
}

#[cfg(target_os = "windows")]
fn is_valid_ssid(ssid: &str) -> bool {
    let value = ssid.trim();
    if value.is_empty() {
        return false;
    }

    if value.contains("未连接") {
        return false;
    }

    let lower = value.to_ascii_lowercase();
    !matches!(
        lower.as_str(),
        "n/a" | "<not connected>" | "not connected" | "disconnected"
    ) && !is_probably_garbled_text(value)
}

fn normalize_wifi_name(ssid: &str) -> String {
    normalize_display_name(ssid).unwrap_or_else(|| ssid.trim().trim_matches('"').trim().to_owned())
}

#[cfg(target_os = "windows")]
async fn collect_windows_connection_profiles() -> HashMap<String, String> {
    let command = [
        "Get-NetConnectionProfile -ErrorAction SilentlyContinue |",
        "ForEach-Object { \"$($_.InterfaceAlias)|$($_.Name)\" }",
    ]
    .join(" ");

    let Some(stdout) = run_windows_powershell(&command).await else {
        return HashMap::new();
    };

    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('|');
            let alias = parts.next()?.trim();
            let profile = parts.next()?.trim();

            (!alias.is_empty() && !profile.is_empty())
                .then(|| (alias.to_owned(), profile.to_owned()))
        })
        .collect()
}

#[cfg(target_os = "windows")]
async fn run_windows_powershell(script: &str) -> Option<String> {
    let wrapped = format!(
        "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); \
         $OutputEncoding = [Console]::OutputEncoding; \
         {script}"
    );

    let mut command = Command::new("powershell");
    command.args(["-NoProfile", "-Command", &wrapped]);
    process::hide_console_window_tokio(&mut command);
    let output = command.output().await.ok()?;

    if !output.status.success() {
        return None;
    }

    decode_powershell_output(&output.stdout)
}

#[cfg(target_os = "windows")]
fn decode_powershell_output(stdout: &[u8]) -> Option<String> {
    let bytes = strip_utf8_bom(stdout);
    if bytes.is_empty() {
        return Some(String::new());
    }

    if looks_like_utf16le(bytes) {
        let utf16_bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
        let utf16 = utf16_bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        return String::from_utf16(&utf16).ok();
    }

    Some(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(target_os = "windows")]
fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes)
}

#[cfg(target_os = "windows")]
fn looks_like_utf16le(bytes: &[u8]) -> bool {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        return true;
    }

    if bytes.len() < 4 {
        return false;
    }

    let sample = bytes.len().min(128);
    let mut zero_bytes = 0usize;
    let mut inspected_pairs = 0usize;
    for chunk in bytes[..sample].chunks(2) {
        if chunk.len() < 2 {
            break;
        }
        inspected_pairs += 1;
        if chunk[1] == 0 {
            zero_bytes += 1;
        }
    }

    inspected_pairs >= 4 && zero_bytes * 2 >= inspected_pairs
}

#[cfg(not(target_os = "windows"))]
async fn detect_unix_like_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = collect_unix_interfaces_from_ip().await;
    if interfaces.is_empty() {
        interfaces = collect_unix_interfaces_from_ifconfig().await;
    }
    let interfaces = dedupe_interfaces(interfaces);

    if interfaces.is_empty() {
        fallback_default_interface()
    } else {
        interfaces
    }
}

#[cfg(not(target_os = "windows"))]
async fn collect_unix_interfaces_from_ip() -> Vec<NetworkInterface> {
    let Ok(output) = Command::new("ip")
        .args(["-o", "-4", "addr", "show", "up", "scope", "global"])
        .output()
        .await
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut interfaces = Vec::new();

    for line in stdout.lines() {
        let Some((alias, ip, prefix)) = parse_unix_ip_addr_row(line) else {
            continue;
        };
        let iface_type = classify_interface(alias);
        if let Some(interface) = build_interface(alias, alias, ip, prefix, iface_type, None) {
            interfaces.push(interface);
        }
    }

    interfaces
}

#[cfg(not(target_os = "windows"))]
async fn collect_unix_interfaces_from_ifconfig() -> Vec<NetworkInterface> {
    let Ok(output) = Command::new("ifconfig").args(["-a"]).output().await else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut interfaces = Vec::new();

    for (alias, ip, prefix) in parse_ifconfig_ipv4_rows(stdout.as_ref()) {
        let iface_type = classify_interface(alias.as_str());
        if let Some(interface) = build_interface(
            alias.as_str(),
            alias.as_str(),
            ip.as_str(),
            prefix,
            iface_type,
            None,
        ) {
            interfaces.push(interface);
        }
    }

    interfaces
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_unix_ip_addr_row(line: &str) -> Option<(&str, &str, u8)> {
    let mut parts = line.split_whitespace();
    let _index = parts.next()?;
    let raw_name = parts.next()?;
    let family = parts.next()?;
    let cidr = parts.next()?;

    if family != "inet" {
        return None;
    }

    let alias = raw_name.split('@').next().unwrap_or(raw_name);
    let mut cidr_parts = cidr.split('/');
    let ip = cidr_parts.next()?;
    let prefix = cidr_parts.next()?.parse::<u8>().ok()?;

    Some((alias, ip, prefix))
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_ifconfig_ipv4_rows(stdout: &str) -> Vec<(String, String, u8)> {
    let mut rows = Vec::new();
    let mut current_alias: Option<String> = None;

    for line in stdout.lines() {
        if let Some(alias) = parse_ifconfig_header_alias(line) {
            current_alias = Some(alias.to_owned());
            continue;
        }

        let Some(alias) = current_alias.as_deref() else {
            continue;
        };
        let Some((ip, prefix)) = parse_ifconfig_inet_row(line) else {
            continue;
        };
        rows.push((alias.to_owned(), ip.to_owned(), prefix));
    }

    rows
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_ifconfig_header_alias(line: &str) -> Option<&str> {
    if line.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    let alias = line
        .split_whitespace()
        .next()
        .map(|segment| segment.trim_end_matches(':'))
        .unwrap_or_default();
    if alias.is_empty() { None } else { Some(alias) }
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_ifconfig_inet_row(line: &str) -> Option<(&str, u8)> {
    let line = line.trim();
    if !line.starts_with("inet ") {
        return None;
    }

    let parts = line.split_whitespace().collect::<Vec<_>>();
    let ip = parts
        .get(1)
        .copied()
        .and_then(|value| value.strip_prefix("addr:").or(Some(value)))
        .filter(|value| !value.is_empty())?;
    let prefix = parse_ifconfig_prefix(parts.as_slice())?;
    Some((ip, prefix))
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_ifconfig_prefix(parts: &[&str]) -> Option<u8> {
    for (index, token) in parts.iter().enumerate() {
        if token.eq_ignore_ascii_case("netmask") {
            let raw = parts.get(index + 1).copied().unwrap_or_default();
            return parse_ifconfig_netmask_prefix(raw);
        }
        if let Some(raw) = token
            .strip_prefix("Mask:")
            .or_else(|| token.strip_prefix("mask:"))
            .or_else(|| token.strip_prefix("netmask:"))
        {
            return parse_ifconfig_netmask_prefix(raw);
        }
    }

    None
}

#[cfg(any(test, not(target_os = "windows")))]
fn parse_ifconfig_netmask_prefix(raw: &str) -> Option<u8> {
    let raw = raw
        .trim()
        .trim_matches(|ch| matches!(ch, ',' | ';' | ')' | '('));
    if raw.is_empty() {
        return None;
    }

    if let Ok(mask) = raw.parse::<Ipv4Addr>() {
        return prefix_from_netmask(u32::from(mask));
    }

    let hex = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    if hex.is_empty() || hex.len() > 8 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }

    let mask = u32::from_str_radix(hex, 16).ok()?;
    prefix_from_netmask(mask)
}

#[cfg(any(test, not(target_os = "windows")))]
fn prefix_from_netmask(mask: u32) -> Option<u8> {
    let prefix = mask.leading_ones();
    let normalized = if prefix == 0 {
        0
    } else {
        u32::MAX << (u32::BITS - prefix)
    };
    (mask == normalized).then_some(prefix as u8)
}

#[cfg(all(unix, not(target_os = "windows"), not(target_os = "macos")))]
async fn collect_unix_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    neighbor_rows::collect_unix_neighbor_rows().await
}

#[cfg(any(test, all(unix, not(target_os = "windows"), not(target_os = "macos"))))]
#[cfg_attr(not(test), allow(dead_code))]
fn parse_unix_neighbor_row(line: &str) -> Option<NeighborEvidenceRow> {
    neighbor_rows::parse_unix_neighbor_row(line)
}

#[cfg(target_os = "macos")]
async fn collect_macos_neighbor_rows() -> Vec<NeighborEvidenceRow> {
    neighbor_rows::collect_macos_neighbor_rows().await
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_arp_row(line: &str) -> Option<NeighborEvidenceRow> {
    neighbor_rows::parse_macos_arp_row(line)
}

fn build_interface(
    id: &str,
    raw_name: &str,
    local_ip: &str,
    prefix_len: u8,
    iface_type: InterfaceType,
    preferred_name: Option<&str>,
) -> Option<NetworkInterface> {
    let ip = local_ip.parse::<Ipv4Addr>().ok()?;
    if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() {
        return None;
    }

    let cleaned_id = normalize_interface_id(id, iface_type)?;

    Some(NetworkInterface {
        id: cleaned_id,
        name: friendly_name(raw_name, iface_type, preferred_name),
        ip_range: network_cidr(ip, prefix_len)?,
        iface_type,
        local_ip: ip.to_string(),
    })
}

fn dedupe_interfaces(interfaces: Vec<NetworkInterface>) -> Vec<NetworkInterface> {
    let mut deduped = HashMap::new();

    for interface in interfaces {
        deduped.entry(interface.id.clone()).or_insert(interface);
    }

    let mut values: Vec<_> = deduped.into_values().collect();
    values.sort_by(|left, right| left.id.cmp(&right.id));
    values
}

fn classify_interface(name: &str) -> InterfaceType {
    let name = name.to_ascii_lowercase();

    if name.contains("wi-fi")
        || name.contains("wifi")
        || name.contains("wlan")
        || name.starts_with("wl")
    {
        InterfaceType::Wifi
    } else if name.contains("docker") || name.contains("vethernet") || name.contains("br-") {
        InterfaceType::Docker
    } else if name.contains("ethernet")
        || name.contains("以太网")
        || name.starts_with("en")
        || name.starts_with("eth")
    {
        InterfaceType::Ethernet
    } else {
        InterfaceType::Other
    }
}

fn friendly_name(
    raw_name: &str,
    iface_type: InterfaceType,
    preferred_name: Option<&str>,
) -> String {
    let fallback = normalize_display_name(raw_name)
        .unwrap_or_else(|| default_interface_name(iface_type).to_owned());

    if let Some(preferred_name) = preferred_name.and_then(normalize_display_name) {
        return match iface_type {
            InterfaceType::Wifi => normalize_wifi_name(&preferred_name),
            _ => preferred_name,
        };
    }

    fallback
}

fn normalize_interface_id(id: &str, iface_type: InterfaceType) -> Option<String> {
    if let Some(clean_id) = normalize_display_name(id) {
        return Some(clean_id);
    }

    trim_display_name(id).map(|_| default_interface_name(iface_type).to_owned())
}

fn normalize_display_name(value: &str) -> Option<String> {
    let value = trim_display_name(value)?;

    if let Some(repaired) = try_repair_mojibake(value) {
        return Some(repaired);
    }

    if value.chars().any(|ch| ch == '\0' || ch.is_control()) {
        return None;
    }

    if is_probably_garbled_text(value) {
        None
    } else {
        Some(value.to_owned())
    }
}

fn is_probably_garbled_text(value: &str) -> bool {
    contains_garbled_sentinel(value)
        || (looks_like_mojibake(value) && try_repair_mojibake(value).is_none())
}

fn try_repair_mojibake(value: &str) -> Option<String> {
    if !looks_like_mojibake(value) {
        return None;
    }

    let bytes = value
        .chars()
        .map(latin1_fallback_byte)
        .collect::<Option<Vec<_>>>()?;
    let decoded = String::from_utf8(bytes).ok()?;
    let decoded = trim_display_name(&decoded)?;

    if decoded == value || contains_garbled_sentinel(decoded) || looks_like_mojibake(decoded) {
        None
    } else {
        Some(decoded.to_owned())
    }
}

fn latin1_fallback_byte(ch: char) -> Option<u8> {
    if let Ok(byte) = u8::try_from(ch as u32) {
        return Some(byte);
    }

    match ch {
        '\u{2018}' => Some(0x91),
        '\u{2019}' => Some(0x92),
        '\u{201C}' => Some(0x93),
        '\u{201D}' => Some(0x94),
        '\u{2026}' => Some(0x85),
        _ => None,
    }
}

fn trim_display_name(value: &str) -> Option<&str> {
    let value = value.trim().trim_matches('"').trim();
    (!value.is_empty()).then_some(value)
}

fn contains_garbled_sentinel(value: &str) -> bool {
    value.contains(char::REPLACEMENT_CHARACTER) || value.contains("锟斤拷")
}

fn looks_like_mojibake(value: &str) -> bool {
    if value.contains("Ã") || value.contains("Â") || value.contains("â€") {
        return true;
    }

    let mut run = 0usize;
    for ch in value.chars() {
        if is_latin1_supplement(ch) {
            run += 1;
            if run >= 3 {
                return true;
            }
        } else {
            run = 0;
        }
    }

    false
}

fn is_latin1_supplement(ch: char) -> bool {
    matches!(ch as u32, 0x0080..=0x00FF)
}

fn default_interface_name(iface_type: InterfaceType) -> &'static str {
    match iface_type {
        InterfaceType::Wifi => "Wi-Fi 网络",
        InterfaceType::Ethernet => "有线网络",
        InterfaceType::Docker => "Docker 网络",
        InterfaceType::Other => "网络接口",
    }
}

#[cfg(any(target_os = "windows", test))]
enum WifiCandidate<'a> {
    Exclude,
    Keep { preferred_name: Option<&'a str> },
}

#[cfg(any(target_os = "windows", test))]
fn resolve_wifi_candidate<'a>(
    observation: Option<&'a WifiObservation>,
    profile_name: Option<&'a str>,
) -> WifiCandidate<'a> {
    match observation {
        Some(WifiObservation {
            availability: WifiAvailability::ExplicitlyUnavailable,
            ..
        }) => WifiCandidate::Exclude,
        Some(observation) => WifiCandidate::Keep {
            preferred_name: observation.ssid.as_deref().or(profile_name),
        },
        None => WifiCandidate::Keep {
            preferred_name: profile_name,
        },
    }
}

fn merge_neighbor_evidence(current: &mut NeighborEvidence, incoming: NeighborEvidence) {
    if current.mac_address.is_none() {
        current.mac_address = incoming.mac_address;
    }
    if current.hostname.is_none() {
        current.hostname = incoming.hostname;
    }
    if current.mdns_name.is_none() {
        current.mdns_name = incoming.mdns_name;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ipv4Subnet {
    network: u32,
    mask: u32,
    prefix_len: u8,
}

impl Ipv4Subnet {
    fn parse(cidr: &str) -> Option<Self> {
        let (ip, prefix_len) = cidr.split_once('/')?;
        let ip = ip.trim().parse::<Ipv4Addr>().ok()?;
        let prefix_len = prefix_len.trim().parse::<u8>().ok()?;
        if prefix_len > 32 {
            return None;
        }

        let mask = if prefix_len == 0 {
            0
        } else {
            u32::MAX << (32 - u32::from(prefix_len))
        };
        let network = u32::from(ip) & mask;
        Some(Self {
            network,
            mask,
            prefix_len,
        })
    }

    fn contains_host(&self, ip: Ipv4Addr) -> bool {
        let ip = u32::from(ip);
        if (ip & self.mask) != self.network {
            return false;
        }

        if self.prefix_len < 31 {
            let broadcast = self.network | !self.mask;
            if ip == self.network || ip == broadcast {
                return false;
            }
        }

        true
    }

    fn host_range(&self) -> Option<(u32, u32)> {
        let broadcast = self.network | !self.mask;
        if self.prefix_len < 31 {
            if broadcast <= self.network + 1 {
                return None;
            }
            Some((self.network + 1, broadcast - 1))
        } else {
            Some((self.network, broadcast))
        }
    }
}

fn parse_ipv4_subnet(cidr: &str) -> Option<Ipv4Subnet> {
    Ipv4Subnet::parse(cidr)
}

fn parse_neighbor_candidate_ip(ip: &str) -> Option<Ipv4Addr> {
    let ip = ip.parse::<Ipv4Addr>().ok()?;
    (!ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()).then_some(ip)
}

fn compare_neighbor_candidate_ip(left: &str, right: &str) -> Ordering {
    match (left.parse::<Ipv4Addr>(), right.parse::<Ipv4Addr>()) {
        (Ok(left), Ok(right)) => left.octets().cmp(&right.octets()),
        _ => left.cmp(right),
    }
}

fn build_neighbor_candidates(
    rows: Vec<NeighborEvidenceRow>,
    local_ip: Ipv4Addr,
    subnet: Ipv4Subnet,
) -> Vec<NeighborCandidate> {
    let mut evidence_by_ip: HashMap<Ipv4Addr, NeighborEvidence> = HashMap::new();

    for row in rows {
        let Some(ip) = parse_neighbor_candidate_ip(&row.ip) else {
            continue;
        };
        if ip == local_ip || !subnet.contains_host(ip) {
            continue;
        }

        evidence_by_ip
            .entry(ip)
            .and_modify(|current| merge_neighbor_evidence(current, row.evidence.clone()))
            .or_insert(row.evidence);
    }

    let mut ordered = evidence_by_ip.into_iter().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|(ip, _)| ip.octets());
    ordered
        .into_iter()
        .map(|(ip, evidence)| NeighborCandidate {
            ip: ip.to_string(),
            evidence,
        })
        .collect()
}

fn refresh_neighbor_candidates_after_priming(
    current_candidates: Vec<NeighborCandidate>,
    rows_after_priming: Vec<NeighborEvidenceRow>,
    local_ip: Ipv4Addr,
    subnet: Ipv4Subnet,
) -> Vec<NeighborCandidate> {
    let mut merged_candidates = current_candidates;
    merged_candidates.extend(build_neighbor_candidates(
        rows_after_priming,
        local_ip,
        subnet,
    ));

    let (ordered_ips, mut evidence_by_ip) = split_neighbor_candidates(merged_candidates);
    ordered_ips
        .into_iter()
        .filter_map(|ip| {
            evidence_by_ip
                .remove(ip.as_str())
                .map(|evidence| NeighborCandidate { ip, evidence })
        })
        .collect()
}

async fn attempt_neighbor_priming(local_ip: Ipv4Addr, subnet: Ipv4Subnet) {
    neighbor_cache::attempt_neighbor_priming(local_ip, subnet).await;
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_neighbor_priming_targets(
    local_ip: Ipv4Addr,
    subnet: Ipv4Subnet,
    max_targets: usize,
) -> Vec<Ipv4Addr> {
    neighbor_cache::build_neighbor_priming_targets(local_ip, subnet, max_targets)
}

#[cfg_attr(not(test), allow(dead_code))]
fn neighbor_candidate_cache_path(local_ip: &str, subnet: &str) -> std::path::PathBuf {
    neighbor_cache::neighbor_candidate_cache_path(local_ip, subnet)
}

fn write_neighbor_candidate_cache(
    local_ip: &str,
    subnet: &str,
    candidates: &[NeighborCandidate],
) -> std::io::Result<()> {
    neighbor_cache::write_neighbor_candidate_cache(local_ip, subnet, candidates)
}

fn remove_neighbor_candidate_cache(local_ip: &str, subnet: &str) {
    neighbor_cache::remove_neighbor_candidate_cache(local_ip, subnet);
}

async fn prime_neighbor_candidate_cache(interfaces: &[NetworkInterface]) {
    neighbor_cache::prime_neighbor_candidate_cache(interfaces).await;
}

fn network_cidr(ip: Ipv4Addr, prefix_len: u8) -> Option<String> {
    if prefix_len > 32 {
        return None;
    }

    let mask = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix_len))
    };
    let network = u32::from(ip) & mask;

    Some(format!("{}/{prefix_len}", Ipv4Addr::from(network)))
}

fn fallback_default_interface() -> Vec<NetworkInterface> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok();

    if let Some(socket) = socket {
        let _ = socket.connect((Ipv4Addr::new(1, 1, 1, 1), 80));

        if let Ok(local_addr) = socket.local_addr()
            && let std::net::IpAddr::V4(ip) = local_addr.ip()
            && let Some(ip_range) = network_cidr(ip, 24)
        {
            return vec![NetworkInterface {
                id: String::from("default"),
                name: String::from("Default Network"),
                ip_range,
                iface_type: InterfaceType::Other,
                local_ip: ip.to_string(),
            }];
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_wifi_names_with_numeric_suffix() {
        assert_eq!(
            friendly_name("Wi-Fi", InterfaceType::Wifi, Some("Yahboom2 3")),
            "Yahboom2 3"
        );
    }

    #[test]
    fn falls_back_to_alias_when_profile_name_is_garbled() {
        assert_eq!(
            friendly_name(
                "VMware Network Adapter VMnet1",
                InterfaceType::Other,
                Some("����")
            ),
            "VMware Network Adapter VMnet1"
        );
    }

    #[test]
    fn repairs_utf8_latin1_mojibake_before_display() {
        assert_eq!(
            normalize_display_name("ä»¥å¤ªç½‘"),
            Some(String::from("以太网"))
        );
    }

    #[test]
    fn treats_unrepairable_mojibake_as_unusable_display_name() {
        assert_eq!(normalize_display_name("ÃÃÃ"), None);
    }

    #[test]
    fn wifi_candidate_keeps_interface_when_ssid_parse_is_missing() {
        assert!(matches!(
            resolve_wifi_candidate(None, Some("Office Network")),
            WifiCandidate::Keep {
                preferred_name: Some("Office Network")
            }
        ));
    }

    #[test]
    fn wifi_candidate_filters_only_explicitly_unavailable_interfaces() {
        let observation = WifiObservation {
            availability: WifiAvailability::ExplicitlyUnavailable,
            ssid: None,
        };

        assert!(matches!(
            resolve_wifi_candidate(Some(&observation), Some("Profile")),
            WifiCandidate::Exclude
        ));
    }

    #[test]
    fn wifi_candidate_keeps_connected_wifi_even_without_ssid() {
        let observation = WifiObservation {
            availability: WifiAvailability::Connected,
            ssid: None,
        };

        assert!(matches!(
            resolve_wifi_candidate(Some(&observation), Some("Office Network")),
            WifiCandidate::Keep {
                preferred_name: Some("Office Network")
            }
        ));
    }

    #[test]
    fn filters_link_local_addresses_from_scan_candidates() {
        assert!(
            build_interface(
                "Wi-Fi",
                "Wi-Fi",
                "169.254.10.2",
                16,
                InterfaceType::Wifi,
                Some("Yahboom4")
            )
            .is_none()
        );
    }

    #[test]
    fn canonical_windows_snapshot_is_order_insensitive() {
        let first = vec![
            String::from("Wi-Fi|192.168.1.10|24"),
            String::from("Ethernet|10.0.0.3|24"),
        ];
        let second = vec![
            String::from("Ethernet|10.0.0.3|24"),
            String::from("Wi-Fi|192.168.1.10|24"),
        ];
        let wifi_first = HashMap::from([(
            String::from("Wi-Fi"),
            WifiObservation {
                availability: WifiAvailability::Connected,
                ssid: Some(String::from("Office Network")),
            },
        )]);
        let wifi_second = HashMap::from([(
            String::from("Wi-Fi"),
            WifiObservation {
                availability: WifiAvailability::Connected,
                ssid: Some(String::from("Office Network")),
            },
        )]);
        let profiles_first = HashMap::from([
            (String::from("Ethernet"), String::from("Wired")),
            (String::from("Wi-Fi"), String::from("Office Network")),
        ]);
        let profiles_second = HashMap::from([
            (String::from("Wi-Fi"), String::from("Office Network")),
            (String::from("Ethernet"), String::from("Wired")),
        ]);

        assert_eq!(
            build_windows_refresh_inputs(&first, &wifi_first, &profiles_first),
            build_windows_refresh_inputs(&second, &wifi_second, &profiles_second)
        );
    }

    #[test]
    fn build_windows_interfaces_filters_explicitly_unavailable_wifi_rows() {
        let rows = vec![
            String::from("Wi-Fi|192.168.31.8|24"),
            String::from("Ethernet|10.0.0.8|24"),
        ];
        let wifi_observations = HashMap::from([(
            String::from("Wi-Fi"),
            WifiObservation {
                availability: WifiAvailability::ExplicitlyUnavailable,
                ssid: None,
            },
        )]);
        let profiles = HashMap::from([(String::from("Ethernet"), String::from("Wired Profile"))]);

        let interfaces = build_windows_interfaces_from_rows(&rows, &wifi_observations, &profiles);
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].id, "Ethernet");
    }

    #[test]
    fn windows_refresh_inputs_change_when_wifi_or_profile_changes() {
        let rows = vec![String::from("Wi-Fi|192.168.31.8|24")];
        let wifi_connected = HashMap::from([(
            String::from("Wi-Fi"),
            WifiObservation {
                availability: WifiAvailability::Connected,
                ssid: Some(String::from("Office Network")),
            },
        )]);
        let wifi_unavailable = HashMap::from([(
            String::from("Wi-Fi"),
            WifiObservation {
                availability: WifiAvailability::ExplicitlyUnavailable,
                ssid: None,
            },
        )]);
        let profile_a = HashMap::from([(String::from("Wi-Fi"), String::from("Office Network"))]);
        let profile_b = HashMap::from([(String::from("Wi-Fi"), String::from("Guest Network"))]);

        let base = build_windows_refresh_inputs(&rows, &wifi_connected, &profile_a);
        assert_ne!(
            base,
            build_windows_refresh_inputs(&rows, &wifi_unavailable, &profile_a)
        );
        assert_ne!(
            base,
            build_windows_refresh_inputs(&rows, &wifi_connected, &profile_b)
        );
    }

    #[test]
    fn parses_windows_neighbor_rows_with_mac_only() {
        let parsed =
            parse_windows_neighbor_row("192.168.31.10|00-04-4B-11-22-33|Reachable").unwrap();
        assert_eq!(parsed.ip, "192.168.31.10");
        assert_eq!(
            parsed.evidence.mac_address.as_deref(),
            Some("00:04:4B:11:22:33")
        );
        assert!(parsed.evidence.hostname.is_none());
    }

    #[test]
    fn ignores_windows_neighbor_rows_with_unusable_state() {
        assert!(
            parse_windows_neighbor_row("192.168.31.10|00-04-4B-11-22-33|Unreachable").is_none()
        );
    }

    #[test]
    fn parses_unix_neighbor_rows_from_ip_command_output() {
        let parsed =
            parse_unix_neighbor_row("192.168.31.14 dev wlan0 lladdr b8:27:eb:11:22:33 REACHABLE")
                .unwrap();
        assert_eq!(parsed.ip, "192.168.31.14");
        assert_eq!(
            parsed.evidence.mac_address.as_deref(),
            Some("B8:27:EB:11:22:33")
        );
    }

    #[test]
    fn ignores_unix_failed_neighbor_rows() {
        assert!(parse_unix_neighbor_row("192.168.31.88 dev wlan0 FAILED").is_none());
    }

    #[test]
    fn parses_macos_arp_row_with_hostname_as_auxiliary_evidence() {
        let parsed =
            parse_macos_arp_row("pi.local (192.168.31.5) at b8:27:eb:11:22:33 on en0").unwrap();
        assert_eq!(parsed.ip, "192.168.31.5");
        assert_eq!(parsed.evidence.hostname.as_deref(), Some("pi.local"));
        assert_eq!(parsed.evidence.mdns_name.as_deref(), Some("pi.local"));
    }

    #[test]
    fn ignores_macos_arp_rows_without_mac() {
        assert!(parse_macos_arp_row("? (192.168.31.5) at (incomplete) on en0").is_none());
    }

    #[test]
    fn parses_linux_style_arp_row_for_macos_parser_path() {
        let parsed =
            parse_macos_arp_row("? (192.168.31.6) at b8:27:eb:11:22:34 [ether] on eth0").unwrap();
        assert_eq!(parsed.ip, "192.168.31.6");
        assert_eq!(
            parsed.evidence.mac_address.as_deref(),
            Some("B8:27:EB:11:22:34")
        );
        assert!(parsed.evidence.hostname.is_none());
    }

    #[test]
    fn parses_unix_ip_addr_row_with_interface_alias_suffix() {
        let (alias, ip, prefix) = parse_unix_ip_addr_row(
            "2: en0@if5    inet 192.168.31.8/24 brd 192.168.31.255 scope global en0",
        )
        .unwrap();
        assert_eq!(alias, "en0");
        assert_eq!(ip, "192.168.31.8");
        assert_eq!(prefix, 24);
    }

    #[test]
    fn parses_ifconfig_row_with_macos_hex_netmask() {
        let (ip, prefix) = parse_ifconfig_inet_row(
            "\tinet 192.168.31.8 netmask 0xffffff00 broadcast 192.168.31.255",
        )
        .unwrap();
        assert_eq!(ip, "192.168.31.8");
        assert_eq!(prefix, 24);
    }

    #[test]
    fn parses_ifconfig_row_with_legacy_linux_mask_format() {
        let (ip, prefix) = parse_ifconfig_inet_row(
            "inet addr:192.168.31.8 Bcast:192.168.31.255 Mask:255.255.255.0",
        )
        .unwrap();
        assert_eq!(ip, "192.168.31.8");
        assert_eq!(prefix, 24);
    }

    #[test]
    fn parses_ifconfig_output_into_multiple_rows() {
        let rows = parse_ifconfig_ipv4_rows(
            "en0: flags=8863<UP,BROADCAST>\n\tinet 192.168.31.8 netmask 0xffffff00 broadcast 192.168.31.255\nlo0: flags=8049<UP,LOOPBACK>\n\tinet 127.0.0.1 netmask 0xff000000\n",
        );
        assert_eq!(
            rows,
            vec![
                (String::from("en0"), String::from("192.168.31.8"), 24),
                (String::from("lo0"), String::from("127.0.0.1"), 8),
            ]
        );
    }

    #[test]
    fn merge_neighbor_evidence_keeps_existing_fields() {
        let mut current = NeighborEvidence::new(
            Some(String::from("B8:27:EB:11:22:33")),
            Some(String::from("pi.local")),
            None,
        );
        let incoming = NeighborEvidence::new(
            Some(String::from("00:04:4B:11:22:33")),
            Some(String::from("jetson.local")),
            Some(String::from("jetson.local")),
        );

        merge_neighbor_evidence(&mut current, incoming);
        assert_eq!(current.mac_address.as_deref(), Some("B8:27:EB:11:22:33"));
        assert_eq!(current.hostname.as_deref(), Some("pi.local"));
        assert_eq!(current.mdns_name.as_deref(), Some("jetson.local"));
    }

    #[test]
    fn builds_neighbor_candidates_with_filter_dedupe_and_stable_sorting() {
        let subnet = parse_ipv4_subnet("192.168.31.8/24").unwrap();
        let rows = vec![
            NeighborEvidenceRow {
                ip: String::from("192.168.31.12"),
                evidence: NeighborEvidence::new(None, Some(String::from("node-12")), None),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.4"),
                evidence: NeighborEvidence::new(
                    Some(String::from("B8:27:EB:11:22:33")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.4"),
                evidence: NeighborEvidence::new(None, Some(String::from("pi.local")), None),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.8"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:33")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.0"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:44")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.255"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:55")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("10.0.0.8"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:66")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("169.254.2.9"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:77")),
                    None,
                    None,
                ),
            },
        ];

        let candidates =
            build_neighbor_candidates(rows, "192.168.31.8".parse::<Ipv4Addr>().unwrap(), subnet);
        let ordered_ips = candidates
            .iter()
            .map(|candidate| candidate.ip.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ordered_ips, vec!["192.168.31.4", "192.168.31.12"]);
        assert_eq!(
            candidates[0].evidence.mac_address.as_deref(),
            Some("B8:27:EB:11:22:33")
        );
        assert_eq!(candidates[0].evidence.hostname.as_deref(), Some("pi.local"));
    }

    #[test]
    fn split_neighbor_candidates_returns_sorted_ips_and_merged_evidence() {
        let candidates = vec![
            NeighborCandidate {
                ip: String::from("192.168.31.44"),
                evidence: NeighborEvidence::new(
                    Some(String::from("B8:27:EB:11:22:33")),
                    None,
                    None,
                ),
            },
            NeighborCandidate {
                ip: String::from("192.168.31.12"),
                evidence: NeighborEvidence::new(None, Some(String::from("desktop-lab")), None),
            },
            NeighborCandidate {
                ip: String::from("192.168.31.44"),
                evidence: NeighborEvidence::new(None, Some(String::from("pi.local")), None),
            },
        ];

        let (ips, evidence_by_ip) = split_neighbor_candidates(candidates);
        assert_eq!(ips, vec!["192.168.31.12", "192.168.31.44"]);
        assert_eq!(
            evidence_by_ip["192.168.31.44"].mac_address.as_deref(),
            Some("B8:27:EB:11:22:33")
        );
        assert_eq!(
            evidence_by_ip["192.168.31.44"].hostname.as_deref(),
            Some("pi.local")
        );
    }

    #[test]
    fn split_neighbor_candidates_handles_empty_input() {
        let (ips, evidence_by_ip) = split_neighbor_candidates(Vec::new());
        assert!(ips.is_empty());
        assert!(evidence_by_ip.is_empty());
    }

    #[test]
    fn parse_ipv4_subnet_normalizes_network_prefix() {
        let subnet = parse_ipv4_subnet("192.168.31.99/24").unwrap();
        assert!(subnet.contains_host("192.168.31.200".parse::<Ipv4Addr>().unwrap()));
        assert!(!subnet.contains_host("192.168.32.1".parse::<Ipv4Addr>().unwrap()));
    }

    #[test]
    fn cache_key_sanitization_is_stable_for_local_ip_and_subnet() {
        let path = neighbor_candidate_cache_path(" 192.168.31.8 ", "192.168.31.0/24");
        let file_name = path.file_name().and_then(|name| name.to_str()).unwrap();
        assert_eq!(
            file_name,
            "sshscanner-neighbor-candidates-192_168_31_8-192_168_31_0_24.txt"
        );
    }

    #[test]
    fn write_neighbor_candidate_cache_persists_ip_lines() {
        let local_ip = "198.18.2.11";
        let subnet = "198.18.2.0/24";
        let path = neighbor_candidate_cache_path(local_ip, subnet);
        let _ = std::fs::remove_file(&path);

        let candidates = vec![
            NeighborCandidate {
                ip: String::from("198.18.2.4"),
                evidence: NeighborEvidence::new(
                    Some(String::from("B8:27:EB:11:22:33")),
                    Some(String::from("pi.local")),
                    None,
                ),
            },
            NeighborCandidate {
                ip: String::from("198.18.2.8"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:33")),
                    None,
                    None,
                ),
            },
        ];
        write_neighbor_candidate_cache(local_ip, subnet, &candidates).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, "198.18.2.4\n198.18.2.8\n");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn build_neighbor_priming_targets_skips_local_and_stays_sorted() {
        let local_ip = "192.168.31.9".parse::<Ipv4Addr>().unwrap();
        let subnet = parse_ipv4_subnet("192.168.31.8/30").unwrap();
        let targets = build_neighbor_priming_targets(local_ip, subnet, 32);
        assert_eq!(targets, vec!["192.168.31.10".parse::<Ipv4Addr>().unwrap()]);
    }

    #[test]
    fn build_neighbor_priming_targets_is_bounded_for_large_subnets() {
        let local_ip = "10.1.0.1".parse::<Ipv4Addr>().unwrap();
        let subnet = parse_ipv4_subnet("10.1.0.1/16").unwrap();
        let targets = build_neighbor_priming_targets(local_ip, subnet, 8);
        assert_eq!(targets.len(), 8);
        assert!(targets.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(!targets.contains(&local_ip));
        assert!(targets.iter().all(|ip| subnet.contains_host(*ip)));
    }

    #[test]
    fn refresh_neighbor_candidates_after_priming_recovers_from_empty_snapshot() {
        let local_ip = "192.168.31.8".parse::<Ipv4Addr>().unwrap();
        let subnet = parse_ipv4_subnet("192.168.31.0/24").unwrap();
        let rows_after_priming = vec![
            NeighborEvidenceRow {
                ip: String::from("192.168.31.12"),
                evidence: NeighborEvidence::new(
                    Some(String::from("B8:27:EB:11:22:33")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.12"),
                evidence: NeighborEvidence::new(None, Some(String::from("pi.local")), None),
            },
        ];

        let candidates = refresh_neighbor_candidates_after_priming(
            Vec::new(),
            rows_after_priming,
            local_ip,
            subnet,
        );
        let ips = candidates
            .iter()
            .map(|candidate| candidate.ip.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ips, vec!["192.168.31.12"]);
        assert_eq!(candidates[0].evidence.hostname.as_deref(), Some("pi.local"));
    }

    #[test]
    fn refresh_neighbor_candidates_after_priming_merges_partial_snapshot_with_new_online_hosts() {
        let local_ip = "192.168.31.8".parse::<Ipv4Addr>().unwrap();
        let subnet = parse_ipv4_subnet("192.168.31.0/24").unwrap();
        let current_candidates = vec![NeighborCandidate {
            ip: String::from("192.168.31.12"),
            evidence: NeighborEvidence::new(None, Some(String::from("pi.local")), None),
        }];
        let rows_after_priming = vec![
            NeighborEvidenceRow {
                ip: String::from("192.168.31.12"),
                evidence: NeighborEvidence::new(
                    Some(String::from("B8:27:EB:11:22:33")),
                    None,
                    None,
                ),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.44"),
                evidence: NeighborEvidence::new(None, Some(String::from("desktop-lab")), None),
            },
            NeighborEvidenceRow {
                ip: String::from("192.168.31.44"),
                evidence: NeighborEvidence::new(
                    Some(String::from("00:04:4B:11:22:33")),
                    None,
                    None,
                ),
            },
        ];

        let candidates = refresh_neighbor_candidates_after_priming(
            current_candidates,
            rows_after_priming,
            local_ip,
            subnet,
        );
        let ips = candidates
            .iter()
            .map(|candidate| candidate.ip.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ips, vec!["192.168.31.12", "192.168.31.44"]);
        assert_eq!(
            candidates[0].evidence.mac_address.as_deref(),
            Some("B8:27:EB:11:22:33")
        );
        assert_eq!(candidates[0].evidence.hostname.as_deref(), Some("pi.local"));
        assert_eq!(
            candidates[1].evidence.mac_address.as_deref(),
            Some("00:04:4B:11:22:33")
        );
        assert_eq!(
            candidates[1].evidence.hostname.as_deref(),
            Some("desktop-lab")
        );
    }
}
