use std::collections::HashMap;
use std::path::PathBuf;

use platform::window::WindowAction;
use ssh_core::credential::store::ToolKind;
use ssh_core::docker::Container;
use ssh_core::network::NetworkInterface;
use ssh_core::scanner::{DeviceStatus, LayeredScanDevice, NeighborEvidence, TcpProbeReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectNoticeTone {
    Success,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ConnectNotice {
    pub tone: ConnectNoticeTone,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
    ToggleLanguage,
    OpenHelpModal,
    CloseHelpModal,
    ShowHelpGuideBasic,
    ShowHelpGuideRustDesk,
    WindowReady(iced::window::Id),
    WindowResized(iced::window::Id),
    WindowMaximizedChanged(bool),
    WindowAction(WindowAction),
    RefreshNetworks,
    NetworksRefreshed(Vec<NetworkInterface>),
    SelectNetwork(String),
    StartScan,
    ScanProgress {
        session_id: u64,
        scanned: usize,
        total: usize,
    },
    ScanDeviceDiscovered {
        session_id: u64,
        device: LayeredScanDevice,
    },
    ScanOnlineDatasetReady {
        session_id: u64,
        evidence_by_ip: HashMap<String, NeighborEvidence>,
    },
    ScanFinished {
        session_id: u64,
    },
    ScanSshProbeFinished {
        session_id: u64,
        report: TcpProbeReport,
    },
    CancelScan,
    SelectDevice(String),
    CloseDetail,
    NetworkDropdownOpened,
    NetworkDropdownClosed,
    UserDropdownOpened,
    UserDropdownClosed,
    SetUsername(String),
    SelectUser(String),
    SetPassword(String),
    ShowAllOnlineResults,
    ShowSshReadyResults,
    ToggleVnc,
    SetVncUser(String),
    SetVncPassword(String),
    OpenCredModal,
    CloseCredModal,
    SetNewCredentialUsername(String),
    SetNewCredentialPassword(String),
    StartEditCredential(String),
    CancelEditCredential,
    AddCredential(String, Option<String>),
    UpdateCredentialPassword(String, String),
    RemoveCredential(String),
    StartVerify,
    VerifyResult {
        session_id: u64,
        ip: String,
        status: DeviceStatus,
    },
    VerifyComplete {
        session_id: u64,
    },
    ConnectShell(String),
    ConnectVSCode(String),
    ConnectMobaXterm(String),
    ConnectVNC(String),
    ConnectDocker(String),
    ConnectRustDesk(String),
    RustDeskProbeFinished {
        device_ip: String,
        password: Option<String>,
        result: Result<(), String>,
    },
    ConnectResult(ToolKind, Result<ConnectNotice, String>),
    DismissNotice(u64),
    RequestToolPath(ToolKind),
    ToolPathPicked(ToolKind, Option<PathBuf>),
    ToolPathPickFailed(ToolKind, String),
    DockerContainersLoaded(Vec<Container>),
    DockerContainersLoadFailed(String),
    SelectContainer(String),
    AttachSelectedContainer,
    CloseDockerModal,
    VisualCheckFrameTick,
    VisualCheckCapture,
    VisualCheckCaptured(iced::window::Screenshot),
    VisualCheckFailed(String),
    OpenGitHub,
    Tick,
    Noop,
}
