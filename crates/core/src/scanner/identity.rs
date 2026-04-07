use super::DeviceType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdentityHint {
    kind: DeviceIdentityKind,
    display_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceIdentityKind {
    RaspberryPi,
    Jetson,
    Computer,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub kind: DeviceIdentityKind,
    pub display_name: String,
    pub device_type: DeviceType,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NeighborEvidence {
    pub mac_address: Option<String>,
    pub hostname: Option<String>,
    pub mdns_name: Option<String>,
}

impl NeighborEvidence {
    pub fn new(
        mac_address: Option<String>,
        hostname: Option<String>,
        mdns_name: Option<String>,
    ) -> Self {
        Self {
            mac_address: normalize_optional_label(mac_address),
            hostname: normalize_optional_label(hostname),
            mdns_name: normalize_optional_label(mdns_name),
        }
    }

    fn auxiliary_label(&self) -> Option<&str> {
        self.mdns_name
            .as_deref()
            .or(self.hostname.as_deref())
            .and_then(trim_label)
    }

    fn label_candidates(&self) -> [Option<&str>; 2] {
        [self.mdns_name.as_deref(), self.hostname.as_deref()]
    }
}

const RDK_BOARD_TOKENS: &[&str] = &["rdk", "drobotics", "hobot"];
const JETSON_BOARD_TOKENS: &[&str] = &["jetson"];
const RASPBERRY_BOARD_TOKENS: &[&str] = &["raspberry", "raspi", "rpi"];
const STRONG_JETSON_OUIS: &[[u8; 3]] =
    &[[0x00, 0x04, 0x4B], [0x3C, 0x6D, 0x66], [0x48, 0xB0, 0x2D]];
const RASPBERRY_VENDOR_KEYWORDS: &[&str] = &["raspberry pi"];
const RDK_VENDOR_KEYWORDS: &[&str] = &["d-robotics", "drobotics", "horizon robotics", "hobot"];
const APPLE_VENDOR_KEYWORDS: &[&str] = &["apple"];
const LENOVO_VENDOR_KEYWORDS: &[&str] = &["lenovo"];
const HP_VENDOR_KEYWORDS: &[&str] = &["hewlett packard", "hp inc", "hp "];
const DELL_VENDOR_KEYWORDS: &[&str] = &["dell"];
const INTEL_VENDOR_KEYWORDS: &[&str] = &["intel"];
const AMD_VENDOR_KEYWORDS: &[&str] = &["advanced micro devices", "amd"];
const ASUS_VENDOR_KEYWORDS: &[&str] = &["asus"];
const ACER_VENDOR_KEYWORDS: &[&str] = &["acer"];
const MSI_VENDOR_KEYWORDS: &[&str] = &["micro-star", "msi"];
const GIGABYTE_VENDOR_KEYWORDS: &[&str] = &["gigabyte"];
const MICROSOFT_VENDOR_KEYWORDS: &[&str] = &["microsoft"];

pub fn classify_device_identity(ip: &str, evidence: Option<&NeighborEvidence>) -> DeviceIdentity {
    classify_device_identity_with_vendor_lookup(ip, evidence, super::oui_db::lookup_vendor_name)
}

fn classify_device_identity_with_vendor_lookup(
    ip: &str,
    evidence: Option<&NeighborEvidence>,
    lookup_vendor_name: impl Fn([u8; 3]) -> Option<&'static str>,
) -> DeviceIdentity {
    let oui_hint = evidence
        .and_then(|item| item.mac_address.as_deref())
        .and_then(oui_prefix)
        .and_then(|prefix| classify_oui(prefix, &lookup_vendor_name));
    let board_label_hint = evidence.and_then(classify_board_label_hint);
    let platform_label_hint = evidence.and_then(classify_platform_label_hint);

    let resolved = resolve_identity_hint(oui_hint, board_label_hint, platform_label_hint);

    let fallback_name = resolved.display_name;
    let display_name = append_auxiliary_label(
        fallback_name,
        evidence.and_then(NeighborEvidence::auxiliary_label),
    );

    let device_type = match resolved.kind {
        DeviceIdentityKind::RaspberryPi | DeviceIdentityKind::Jetson => DeviceType::Server,
        DeviceIdentityKind::Computer | DeviceIdentityKind::Unknown => DeviceType::Desktop,
    };

    let id_name = if display_name.is_empty() {
        format!("Unknown Device ({ip})")
    } else {
        display_name
    };

    DeviceIdentity {
        kind: resolved.kind,
        display_name: id_name,
        device_type,
    }
}

fn resolve_identity_hint(
    oui_hint: Option<IdentityHint>,
    board_label_hint: Option<IdentityHint>,
    platform_label_hint: Option<IdentityHint>,
) -> IdentityHint {
    if let Some(oui_hint) = oui_hint {
        if matches!(
            oui_hint.kind,
            DeviceIdentityKind::RaspberryPi
                | DeviceIdentityKind::Jetson
                | DeviceIdentityKind::Computer
        ) {
            return oui_hint;
        }

        if let Some(board_label_hint) = board_label_hint {
            return board_label_hint;
        }

        return oui_hint;
    }

    if let Some(board_label_hint) = board_label_hint {
        return board_label_hint;
    }

    platform_label_hint.unwrap_or(IdentityHint {
        kind: DeviceIdentityKind::Unknown,
        display_name: default_display_name(DeviceIdentityKind::Unknown),
    })
}

fn append_auxiliary_label(base_name: &str, auxiliary: Option<&str>) -> String {
    match auxiliary.and_then(trim_label) {
        Some(extra) => format!("{base_name} ({extra})"),
        None => base_name.to_owned(),
    }
}

fn default_display_name(identity_kind: DeviceIdentityKind) -> &'static str {
    match identity_kind {
        DeviceIdentityKind::RaspberryPi => "Raspberry Pi",
        DeviceIdentityKind::Jetson => "NVIDIA Jetson",
        DeviceIdentityKind::Computer => "Computer",
        DeviceIdentityKind::Unknown => "Unknown Device",
    }
}

fn classify_oui(
    prefix: [u8; 3],
    lookup_vendor_name: &impl Fn([u8; 3]) -> Option<&'static str>,
) -> Option<IdentityHint> {
    let vendor_name = lookup_vendor_name(prefix)?;

    if STRONG_JETSON_OUIS.contains(&prefix)
        && vendor_name_contains_any(&vendor_name.to_ascii_lowercase(), &["nvidia"])
    {
        return Some(IdentityHint {
            kind: DeviceIdentityKind::Jetson,
            display_name: default_display_name(DeviceIdentityKind::Jetson),
        });
    }

    Some(classify_vendor_name(vendor_name))
}

fn classify_vendor_name(vendor_name: &'static str) -> IdentityHint {
    let normalized_vendor_name = vendor_name.to_ascii_lowercase();

    if vendor_name_contains_any(&normalized_vendor_name, RASPBERRY_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::RaspberryPi,
            display_name: default_display_name(DeviceIdentityKind::RaspberryPi),
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, RDK_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Unknown,
            display_name: "D-Robotics RDK",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, APPLE_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Apple Mac",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, LENOVO_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Lenovo Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, HP_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "HP Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, DELL_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Dell Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, INTEL_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Intel Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, AMD_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "AMD Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, ASUS_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "ASUS Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, ACER_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Acer Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, MSI_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "MSI Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, GIGABYTE_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Gigabyte Computer",
        };
    }

    if vendor_name_contains_any(&normalized_vendor_name, MICROSOFT_VENDOR_KEYWORDS) {
        return IdentityHint {
            kind: DeviceIdentityKind::Computer,
            display_name: "Microsoft Computer",
        };
    }

    IdentityHint {
        kind: DeviceIdentityKind::Unknown,
        display_name: vendor_name,
    }
}

fn classify_board_label_hint(evidence: &NeighborEvidence) -> Option<IdentityHint> {
    let mut saw_rdk = false;
    let mut saw_jetson = false;
    let mut saw_raspberry = false;

    for label in evidence.label_candidates().into_iter().flatten() {
        let normalized = normalize_label_for_match(label);
        saw_rdk |= contains_any_token(&normalized, RDK_BOARD_TOKENS);
        saw_jetson |= contains_any_token(&normalized, JETSON_BOARD_TOKENS);
        saw_raspberry |= contains_any_token_prefix(&normalized, RASPBERRY_BOARD_TOKENS);
    }

    if saw_rdk {
        return Some(IdentityHint {
            kind: DeviceIdentityKind::Unknown,
            display_name: "D-Robotics RDK",
        });
    }

    if saw_jetson && saw_raspberry {
        return None;
    }

    if saw_jetson {
        return Some(IdentityHint {
            kind: DeviceIdentityKind::Jetson,
            display_name: default_display_name(DeviceIdentityKind::Jetson),
        });
    }

    if saw_raspberry {
        return Some(IdentityHint {
            kind: DeviceIdentityKind::RaspberryPi,
            display_name: default_display_name(DeviceIdentityKind::RaspberryPi),
        });
    }

    None
}

fn classify_platform_label_hint(evidence: &NeighborEvidence) -> Option<IdentityHint> {
    for label in evidence.label_candidates().into_iter().flatten() {
        let normalized = normalize_label_for_match(label);

        if contains_any_token(
            &normalized,
            &["macbook", "imac", "macmini", "macstudio", "macpro", "mac"],
        ) {
            return Some(IdentityHint {
                kind: DeviceIdentityKind::Computer,
                display_name: "Apple Mac",
            });
        }

        if contains_any_token(&normalized, &["windows", "win11", "win10", "winpc"]) {
            return Some(IdentityHint {
                kind: DeviceIdentityKind::Computer,
                display_name: "Windows PC",
            });
        }

        if contains_any_token(
            &normalized,
            &[
                "ubuntu",
                "debian",
                "fedora",
                "arch",
                "archlinux",
                "linuxmint",
                "mint",
                "centos",
                "rocky",
                "almalinux",
                "linux",
            ],
        ) {
            return Some(IdentityHint {
                kind: DeviceIdentityKind::Computer,
                display_name: "Linux Computer",
            });
        }

        if contains_any_token(
            &normalized,
            &[
                "desktop",
                "laptop",
                "notebook",
                "workstation",
                "thinkpad",
                "computer",
                "pc",
            ],
        ) {
            return Some(IdentityHint {
                kind: DeviceIdentityKind::Computer,
                display_name: default_display_name(DeviceIdentityKind::Computer),
            });
        }
    }

    None
}

fn oui_prefix(raw_mac: &str) -> Option<[u8; 3]> {
    let hex = raw_mac
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();

    if hex.len() < 12 {
        return None;
    }

    let mut prefix = [0u8; 3];
    for (index, slot) in prefix.iter_mut().enumerate() {
        let offset = index * 2;
        *slot = u8::from_str_radix(&hex[offset..offset + 2], 16).ok()?;
    }

    Some(prefix)
}

fn normalize_optional_label(value: Option<String>) -> Option<String> {
    value.as_deref().and_then(trim_label).map(str::to_owned)
}

fn normalize_label_for_match(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .collect()
}

fn contains_any_token(tokens: &[String], expected: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| expected.iter().any(|item| token == item))
}

fn contains_any_token_prefix(tokens: &[String], expected_prefixes: &[&str]) -> bool {
    tokens.iter().any(|token| {
        expected_prefixes
            .iter()
            .any(|prefix| token.starts_with(prefix))
    })
}

fn vendor_name_contains_any(normalized_vendor_name: &str, keywords: &[&str]) -> bool {
    keywords
        .iter()
        .any(|keyword| normalized_vendor_name.contains(keyword))
}

fn trim_label(value: &str) -> Option<&str> {
    let value = value.trim().trim_matches('"').trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup_vendor_name(prefix: [u8; 3]) -> Option<&'static str> {
        match prefix {
            [0xB8, 0x27, 0xEB] => Some("Raspberry Pi Trading Ltd"),
            [0xDC, 0xA6, 0x32] => Some("Raspberry Pi Trading Ltd"),
            [0x00, 0x04, 0x4B] => Some("NVIDIA CORPORATION"),
            [0x48, 0xB0, 0x2D] => Some("NVIDIA CORPORATION"),
            [0x10, 0x20, 0x30] => Some("NVIDIA CORPORATION"),
            [0xF0, 0x18, 0x98] => Some("Apple, Inc."),
            [0xA4, 0x5E, 0x60] => Some("Apple, Inc."),
            [0x00, 0x00, 0x1A] => Some("Advanced Micro Devices"),
            [0xAA, 0xBB, 0xCC] => Some("Acme Industrial Systems"),
            _ => None,
        }
    }

    fn classify_with_lookup(ip: &str, evidence: Option<&NeighborEvidence>) -> DeviceIdentity {
        classify_device_identity_with_vendor_lookup(ip, evidence, lookup_vendor_name)
    }

    #[test]
    fn vendor_lookup_maps_multiple_prefixes_to_same_family() {
        let raspberry = [
            NeighborEvidence::new(Some(String::from("B8:27:EB:10:20:30")), None, None),
            NeighborEvidence::new(Some(String::from("DC:A6:32:10:20:30")), None, None),
        ];
        for evidence in raspberry {
            let identity = classify_with_lookup("192.168.31.8", Some(&evidence));
            assert_eq!(identity.kind, DeviceIdentityKind::RaspberryPi);
            assert_eq!(identity.display_name, "Raspberry Pi");
            assert_eq!(identity.device_type, DeviceType::Server);
        }

        let jetson = [
            NeighborEvidence::new(Some(String::from("00:04:4B:10:20:30")), None, None),
            NeighborEvidence::new(Some(String::from("48:B0:2D:10:20:30")), None, None),
        ];
        for evidence in jetson {
            let identity = classify_with_lookup("192.168.31.9", Some(&evidence));
            assert_eq!(identity.kind, DeviceIdentityKind::Jetson);
            assert_eq!(identity.display_name, "NVIDIA Jetson");
            assert_eq!(identity.device_type, DeviceType::Server);
        }

        let computer = [
            NeighborEvidence::new(Some(String::from("F0:18:98:10:20:30")), None, None),
            NeighborEvidence::new(Some(String::from("A4:5E:60:10:20:30")), None, None),
        ];
        for evidence in computer {
            let identity = classify_with_lookup("192.168.31.10", Some(&evidence));
            assert_eq!(identity.kind, DeviceIdentityKind::Computer);
            assert_eq!(identity.display_name, "Apple Mac");
            assert_eq!(identity.device_type, DeviceType::Desktop);
        }
    }

    #[test]
    fn vendor_known_but_family_unmapped_falls_back_to_vendor_display() {
        let evidence = NeighborEvidence::new(Some(String::from("AA:BB:CC:10:20:30")), None, None);
        let identity = classify_with_lookup("192.168.31.19", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "Acme Industrial Systems");
        assert_eq!(identity.device_type, DeviceType::Desktop);
    }

    #[test]
    fn unknown_oui_falls_back_to_unknown_device() {
        let evidence = NeighborEvidence::new(Some(String::from("DE:AD:BE:EF:01:02")), None, None);
        let identity = classify_with_lookup("192.168.31.12", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "Unknown Device");
    }

    #[test]
    fn plain_nvidia_vendor_without_strong_board_signal_stays_unknown_safe() {
        let evidence = NeighborEvidence::new(Some(String::from("10:20:30:10:20:30")), None, None);
        let identity = classify_with_lookup("192.168.31.120", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "NVIDIA CORPORATION");
    }

    #[test]
    fn board_label_beats_generic_vendor_oui_when_oui_is_not_board_strong() {
        let evidence = NeighborEvidence::new(
            Some(String::from("10:20:30:10:20:30")),
            Some(String::from("jetson-agx")),
            None,
        );
        let identity = classify_with_lookup("192.168.31.121", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Jetson);
        assert_eq!(identity.display_name, "NVIDIA Jetson (jetson-agx)");
    }

    #[test]
    fn mapped_vendor_oui_beats_conflicting_board_label() {
        let evidence = NeighborEvidence::new(
            Some(String::from("F0:18:98:10:20:30")),
            Some(String::from("jetson-agx")),
            None,
        );
        let identity = classify_with_lookup("192.168.31.122", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Apple Mac (jetson-agx)");
    }

    #[test]
    fn amd_vendor_maps_to_computer_family() {
        let evidence = NeighborEvidence::new(Some(String::from("00:00:1A:10:20:30")), None, None);
        let identity = classify_with_lookup("192.168.31.123", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "AMD Computer");
    }

    #[test]
    fn strong_board_oui_remains_stronger_than_conflicting_weak_labels() {
        let evidence = NeighborEvidence::new(
            Some(String::from("B8:27:EB:10:20:30")),
            Some(String::from("rdk-workstation")),
            Some(String::from("rdk-x5.local")),
        );
        let identity = classify_with_lookup("192.168.31.88", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::RaspberryPi);
        assert_eq!(identity.display_name, "Raspberry Pi (rdk-x5.local)");
    }

    #[test]
    fn weak_label_without_oui_only_upgrades_to_broad_computer_family() {
        let evidence = NeighborEvidence::new(None, Some(String::from("lab-workstation")), None);
        let identity = classify_with_lookup("192.168.31.53", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Computer (lab-workstation)");
    }

    #[test]
    fn raspberry_label_upgrades_when_oui_absent() {
        let evidence = NeighborEvidence::new(None, Some(String::from("raspi5-devkit")), None);
        let identity = classify_with_lookup("192.168.31.87", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::RaspberryPi);
        assert_eq!(identity.display_name, "Raspberry Pi (raspi5-devkit)");
    }

    #[test]
    fn jetson_board_label_beats_generic_platform_label_without_oui() {
        let evidence = NeighborEvidence::new(None, Some(String::from("jetson-workstation")), None);
        let identity = classify_with_lookup("192.168.31.86", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Jetson);
        assert_eq!(identity.display_name, "NVIDIA Jetson (jetson-workstation)");
    }

    #[test]
    fn conflicting_rdk_and_jetson_labels_prefer_unknown_first() {
        let evidence = NeighborEvidence::new(
            Some(String::from("de:ad:be:ef:01:02")),
            Some(String::from("jetson-agx")),
            Some(String::from("rdk-x5.local")),
        );
        let identity = classify_with_lookup("192.168.31.89", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "D-Robotics RDK (rdk-x5.local)");
    }

    #[test]
    fn rdk_label_keeps_unknown_even_with_generic_computer_tokens() {
        let evidence = NeighborEvidence::new(None, Some(String::from("rdk-workstation")), None);
        let identity = classify_with_lookup("192.168.31.21", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "D-Robotics RDK (rdk-workstation)");
    }

    #[test]
    fn generic_robotics_token_does_not_force_rdk_vendor_display() {
        let evidence =
            NeighborEvidence::new(None, Some(String::from("robotics-workstation")), None);
        let identity = classify_with_lookup("192.168.31.22", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Computer (robotics-workstation)");
    }

    #[test]
    fn unknown_first_with_weak_label_only_improves_display_name() {
        let evidence = NeighborEvidence::new(None, Some(String::from("node-12")), None);
        let identity = classify_with_lookup("192.168.31.13", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Unknown);
        assert_eq!(identity.display_name, "Unknown Device (node-12)");
    }

    #[test]
    fn parses_dot_separated_mac_prefix() {
        assert_eq!(oui_prefix("0004.4b01.0203"), Some([0x00, 0x04, 0x4B]));
    }

    #[test]
    fn macbook_label_is_classified_as_apple_mac() {
        let evidence = NeighborEvidence::new(None, Some(String::from("macbook-pro.local")), None);
        let identity = classify_with_lookup("192.168.31.50", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Apple Mac (macbook-pro.local)");
    }

    #[test]
    fn windows_label_is_classified_as_windows_pc() {
        let evidence = NeighborEvidence::new(None, Some(String::from("win11-lab")), None);
        let identity = classify_with_lookup("192.168.31.51", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Windows PC (win11-lab)");
    }

    #[test]
    fn linux_label_is_classified_as_linux_computer() {
        let evidence = NeighborEvidence::new(None, Some(String::from("ubuntu-devbox")), None);
        let identity = classify_with_lookup("192.168.31.52", Some(&evidence));
        assert_eq!(identity.kind, DeviceIdentityKind::Computer);
        assert_eq!(identity.display_name, "Linux Computer (ubuntu-devbox)");
    }
}
