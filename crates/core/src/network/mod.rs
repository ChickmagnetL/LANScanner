use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

pub trait NetworkDetector: Send + Sync + 'static {
    fn detect_interfaces(&self)
    -> Pin<Box<dyn Future<Output = Vec<NetworkInterface>> + Send + '_>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceType {
    Wifi,
    Ethernet,
    Docker,
    Other,
}

impl InterfaceType {
    pub fn short_label(self) -> &'static str {
        match self {
            Self::Wifi => "WiFi",
            Self::Ethernet => "LAN",
            Self::Docker => "Docker",
            Self::Other => "Net",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkInterface {
    pub id: String,
    pub name: String,
    pub ip_range: String,
    pub iface_type: InterfaceType,
    pub local_ip: String,
}

impl Display for NetworkInterface {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} ({})",
            self.iface_type.short_label(),
            self.name,
            self.ip_range
        )
    }
}

static DETECTOR: OnceLock<Arc<dyn NetworkDetector>> = OnceLock::new();

pub fn register_detector(detector: Arc<dyn NetworkDetector>) {
    let _ = DETECTOR.set(detector);
}

pub async fn detect_interfaces() -> Vec<NetworkInterface> {
    match DETECTOR.get() {
        Some(detector) => detector.detect_interfaces().await,
        None => Vec::new(),
    }
}

pub fn signature(interfaces: &[NetworkInterface]) -> u64 {
    let mut sorted = interfaces.to_vec();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));

    let mut hasher = DefaultHasher::new();
    sorted.hash(&mut hasher);
    hasher.finish()
}

pub fn select_by_id<'a>(
    interfaces: &'a [NetworkInterface],
    selected_id: Option<&str>,
) -> Option<&'a NetworkInterface> {
    selected_id.and_then(|selected_id| interfaces.iter().find(|iface| iface.id == selected_id))
}
