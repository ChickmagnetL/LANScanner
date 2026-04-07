mod identity;
mod oui_db;
mod tcp_scan;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;

pub use identity::{
    DeviceIdentity, DeviceIdentityKind, NeighborEvidence, classify_device_identity,
};
pub use tcp_scan::{
    SCAN_CONCURRENCY, SCAN_TIMEOUT, SSH_PORT, TcpProbeReport, scan_subnet, scan_subnet_report,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Untested,
    Ready,
    Denied,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Laptop,
    Server,
    Desktop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub identity_kind: DeviceIdentityKind,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshPortProbeStatus {
    Unchecked,
    Open,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredScanDevice {
    pub device: Device,
    pub ssh_port_status: SshPortProbeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredScanDevices {
    pub online_devices: Vec<LayeredScanDevice>,
    pub ssh_ready_devices: Vec<Device>,
}

pub fn devices_from_ips(ips: Vec<String>) -> Vec<Device> {
    devices_from_identity_evidence(ips, HashMap::new())
}

pub fn devices_from_identity_evidence(
    ips: Vec<String>,
    evidence_by_ip: HashMap<String, NeighborEvidence>,
) -> Vec<Device> {
    let mut devices: Vec<_> = ips
        .into_iter()
        .map(|ip| {
            let evidence = evidence_by_ip.get(ip.as_str());
            device_from_identity_evidence(ip, evidence)
        })
        .collect();

    sort_devices_by_ip(&mut devices);
    devices
}

pub fn device_from_identity_evidence(ip: String, evidence: Option<&NeighborEvidence>) -> Device {
    let identity = classify_device_identity(&ip, evidence);
    Device {
        id: ip.clone(),
        name: identity.display_name,
        ip,
        identity_kind: identity.kind,
        device_type: identity.device_type,
        status: DeviceStatus::Untested,
    }
}

pub fn build_layered_scan_devices(
    online_ips: Vec<String>,
    ssh_open_ips: Vec<String>,
    ssh_closed_ips: Vec<String>,
    evidence_by_ip: HashMap<String, NeighborEvidence>,
) -> LayeredScanDevices {
    build_layered_scan_devices_with_probe_semantics(
        online_ips,
        ssh_open_ips,
        ssh_closed_ips,
        Vec::new(),
        evidence_by_ip,
    )
}

pub fn build_layered_scan_devices_from_probe_report(
    online_ips: Vec<String>,
    probe_report: &TcpProbeReport,
    evidence_by_ip: HashMap<String, NeighborEvidence>,
) -> LayeredScanDevices {
    build_layered_scan_devices_with_probe_semantics(
        online_ips,
        probe_report.open_hosts.clone(),
        probe_report.closed_hosts.clone(),
        probe_report.retry_exhausted_hosts.clone(),
        evidence_by_ip,
    )
}

fn build_layered_scan_devices_with_probe_semantics(
    online_ips: Vec<String>,
    ssh_open_ips: Vec<String>,
    ssh_closed_ips: Vec<String>,
    ssh_indeterminate_ips: Vec<String>,
    evidence_by_ip: HashMap<String, NeighborEvidence>,
) -> LayeredScanDevices {
    let normalized_online_ips = normalize_and_sort_ips(online_ips);
    let online_ip_set: HashSet<&str> = normalized_online_ips.iter().map(String::as_str).collect();

    let normalized_open_ips = normalize_and_sort_ips(ssh_open_ips)
        .into_iter()
        .filter(|ip| online_ip_set.contains(ip.as_str()))
        .collect::<Vec<_>>();
    let ssh_open_set: HashSet<&str> = normalized_open_ips.iter().map(String::as_str).collect();
    let normalized_indeterminate_ips = normalize_and_sort_ips(ssh_indeterminate_ips)
        .into_iter()
        .filter(|ip| online_ip_set.contains(ip.as_str()))
        .filter(|ip| !ssh_open_set.contains(ip.as_str()))
        .collect::<Vec<_>>();
    let ssh_indeterminate_set: HashSet<&str> = normalized_indeterminate_ips
        .iter()
        .map(String::as_str)
        .collect();
    let normalized_closed_ips = normalize_and_sort_ips(ssh_closed_ips)
        .into_iter()
        .filter(|ip| online_ip_set.contains(ip.as_str()))
        .filter(|ip| !ssh_open_set.contains(ip.as_str()))
        .filter(|ip| !ssh_indeterminate_set.contains(ip.as_str()))
        .collect::<Vec<_>>();

    let ssh_closed_set: HashSet<&str> = normalized_closed_ips.iter().map(String::as_str).collect();

    let online_devices = normalized_online_ips
        .into_iter()
        .map(|ip| {
            let evidence = evidence_by_ip.get(ip.as_str());
            let device = device_from_identity_evidence(ip, evidence);
            let ssh_port_status = if ssh_open_set.contains(device.ip.as_str()) {
                SshPortProbeStatus::Open
            } else if ssh_indeterminate_set.contains(device.ip.as_str()) {
                SshPortProbeStatus::Unchecked
            } else if ssh_closed_set.contains(device.ip.as_str()) {
                SshPortProbeStatus::Closed
            } else {
                SshPortProbeStatus::Unchecked
            };

            LayeredScanDevice {
                device,
                ssh_port_status,
            }
        })
        .collect();

    let ssh_ready_devices = normalized_open_ips
        .into_iter()
        .map(|ip| {
            let evidence = evidence_by_ip.get(ip.as_str());
            device_from_identity_evidence(ip, evidence)
        })
        .collect();

    LayeredScanDevices {
        online_devices,
        ssh_ready_devices,
    }
}

pub fn sort_devices_by_ip(devices: &mut [Device]) {
    devices.sort_by(compare_devices_by_ip);
}

pub fn compare_devices_by_ip(left: &Device, right: &Device) -> Ordering {
    compare_ip_and_id(
        left.ip.as_str(),
        left.id.as_str(),
        right.ip.as_str(),
        right.id.as_str(),
    )
}

pub fn prioritize_ready_devices(devices: &mut [Device]) {
    devices.sort_by_key(|device| status_priority(device.status));
}

fn status_priority(status: DeviceStatus) -> u8 {
    match status {
        DeviceStatus::Ready => 0,
        DeviceStatus::Untested | DeviceStatus::Denied | DeviceStatus::Error => 1,
    }
}

fn compare_ip_and_id(left_ip: &str, left_id: &str, right_ip: &str, right_id: &str) -> Ordering {
    match (left_ip.parse::<Ipv4Addr>(), right_ip.parse::<Ipv4Addr>()) {
        (Ok(left_ip), Ok(right_ip)) => left_ip
            .octets()
            .cmp(&right_ip.octets())
            .then_with(|| left_id.cmp(right_id)),
        _ => left_ip.cmp(right_ip).then_with(|| left_id.cmp(right_id)),
    }
}

fn normalize_and_sort_ips(mut ips: Vec<String>) -> Vec<String> {
    ips.sort_by(|left, right| compare_ip_and_id(left, left, right, right));
    ips.dedup();
    ips
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_are_sorted_by_ipv4_value() {
        let devices = devices_from_ips(vec![
            String::from("192.168.31.100"),
            String::from("192.168.31.8"),
            String::from("192.168.31.44"),
        ]);
        let ordered: Vec<_> = devices.into_iter().map(|device| device.ip).collect();
        assert_eq!(
            ordered,
            vec!["192.168.31.8", "192.168.31.44", "192.168.31.100"]
        );
    }

    #[test]
    fn unknown_name_is_not_inferred_from_ip_tail() {
        let devices = devices_from_ips(vec![String::from("192.168.31.12")]);
        assert_eq!(devices[0].name, "Unknown Device");
        assert_eq!(devices[0].identity_kind, DeviceIdentityKind::Unknown);
    }

    #[test]
    fn single_device_builder_honors_unknown_fallback_without_evidence() {
        let device = device_from_identity_evidence(String::from("192.168.31.12"), None);
        assert_eq!(device.id, "192.168.31.12");
        assert_eq!(device.ip, "192.168.31.12");
        assert_eq!(device.name, "Unknown Device");
        assert_eq!(device.identity_kind, DeviceIdentityKind::Unknown);
        assert_eq!(device.status, DeviceStatus::Untested);
    }

    #[test]
    fn sort_helper_keeps_ipv4_numeric_order() {
        let mut devices = vec![
            device_from_identity_evidence(String::from("192.168.31.100"), None),
            device_from_identity_evidence(String::from("192.168.31.8"), None),
            device_from_identity_evidence(String::from("192.168.31.44"), None),
        ];
        sort_devices_by_ip(&mut devices);
        let ordered: Vec<_> = devices.into_iter().map(|device| device.ip).collect();
        assert_eq!(
            ordered,
            vec!["192.168.31.8", "192.168.31.44", "192.168.31.100"]
        );
    }

    #[test]
    fn device_payload_preserves_structured_identity_kind() {
        let mut evidence_by_ip = HashMap::new();
        evidence_by_ip.insert(
            String::from("192.168.31.8"),
            NeighborEvidence::new(Some(String::from("B8:27:EB:11:22:33")), None, None),
        );
        evidence_by_ip.insert(
            String::from("192.168.31.9"),
            NeighborEvidence::new(Some(String::from("00:04:4B:AA:BB:CC")), None, None),
        );

        let devices = devices_from_identity_evidence(
            vec![String::from("192.168.31.8"), String::from("192.168.31.9")],
            evidence_by_ip,
        );
        assert_eq!(devices[0].identity_kind, DeviceIdentityKind::RaspberryPi);
        assert_eq!(devices[1].identity_kind, DeviceIdentityKind::Jetson);
    }

    #[test]
    fn layered_scan_devices_keep_online_and_ssh_views_split() {
        let mut evidence_by_ip = HashMap::new();
        evidence_by_ip.insert(
            String::from("192.168.31.44"),
            NeighborEvidence::new(Some(String::from("B8:27:EB:11:22:33")), None, None),
        );
        evidence_by_ip.insert(
            String::from("192.168.31.12"),
            NeighborEvidence::new(None, Some(String::from("desktop-lab")), None),
        );

        let layered = build_layered_scan_devices(
            vec![
                String::from("192.168.31.44"),
                String::from("192.168.31.12"),
                String::from("192.168.31.44"),
            ],
            vec![
                String::from("192.168.31.44"),
                String::from("192.168.31.200"),
            ],
            vec![String::from("192.168.31.12"), String::from("192.168.31.9")],
            evidence_by_ip,
        );

        let online_ips = layered
            .online_devices
            .iter()
            .map(|entry| entry.device.ip.as_str())
            .collect::<Vec<_>>();
        let probe_states = layered
            .online_devices
            .iter()
            .map(|entry| entry.ssh_port_status)
            .collect::<Vec<_>>();
        let ssh_ready_ips = layered
            .ssh_ready_devices
            .iter()
            .map(|device| device.ip.as_str())
            .collect::<Vec<_>>();

        assert_eq!(online_ips, vec!["192.168.31.12", "192.168.31.44"]);
        assert_eq!(
            probe_states,
            vec![SshPortProbeStatus::Closed, SshPortProbeStatus::Open]
        );
        assert_eq!(ssh_ready_ips, vec!["192.168.31.44"]);
    }

    #[test]
    fn layered_scan_devices_mark_unprobed_online_hosts_as_unchecked() {
        let layered = build_layered_scan_devices(
            vec![String::from("192.168.31.12"), String::from("192.168.31.44")],
            vec![String::from("192.168.31.44")],
            Vec::new(),
            HashMap::new(),
        );

        let probe_states = layered
            .online_devices
            .iter()
            .map(|entry| (entry.device.ip.as_str(), entry.ssh_port_status))
            .collect::<Vec<_>>();

        assert_eq!(
            probe_states,
            vec![
                ("192.168.31.12", SshPortProbeStatus::Unchecked),
                ("192.168.31.44", SshPortProbeStatus::Open),
            ]
        );
    }

    #[test]
    fn layered_scan_devices_from_probe_report_keeps_retry_exhausted_as_unchecked() {
        let probe_report = TcpProbeReport {
            candidate_hosts: vec![
                String::from("192.168.31.12"),
                String::from("192.168.31.44"),
                String::from("192.168.31.50"),
            ],
            open_hosts: vec![String::from("192.168.31.44")],
            closed_hosts: vec![String::from("192.168.31.12"), String::from("192.168.31.50")],
            retry_exhausted_hosts: vec![String::from("192.168.31.12")],
        };
        let layered = build_layered_scan_devices_from_probe_report(
            vec![
                String::from("192.168.31.44"),
                String::from("192.168.31.12"),
                String::from("192.168.31.50"),
            ],
            &probe_report,
            HashMap::new(),
        );
        let probe_states = layered
            .online_devices
            .iter()
            .map(|entry| (entry.device.ip.as_str(), entry.ssh_port_status))
            .collect::<Vec<_>>();

        assert_eq!(
            probe_states,
            vec![
                ("192.168.31.12", SshPortProbeStatus::Unchecked),
                ("192.168.31.44", SshPortProbeStatus::Open),
                ("192.168.31.50", SshPortProbeStatus::Closed),
            ]
        );
    }
}
