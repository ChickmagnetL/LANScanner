use iced::Task;
use ssh_core::credential::store::ToolKind;

use crate::message::Message;

use super::{ActiveQuickConnectState, CONNECT_NOTICE_DISMISS_DELAY, Notice, ShellApp};

impl ShellApp {
    pub(super) fn set_active_quick_connect(
        &mut self,
        device_ip: String,
        launcher_key: &'static str,
    ) {
        self.active_quick_connect = Some(ActiveQuickConnectState {
            device_ip,
            launcher_key,
        });
    }

    pub(super) fn clear_active_quick_connect(&mut self) {
        self.active_quick_connect = None;
    }

    pub(super) fn set_notice(&mut self, notice: Notice) -> u64 {
        self.notice_version = self.notice_version.wrapping_add(1);
        self.notice = Some(notice);
        self.notice_version
    }

    pub(super) fn set_quick_connect_notice(&mut self, notice: Notice) -> Task<Message> {
        let version = self.set_notice(notice);
        Self::schedule_notice_dismiss(version)
    }

    pub(super) fn clear_notice(&mut self) {
        self.notice_version = self.notice_version.wrapping_add(1);
        self.notice = None;
    }

    pub(super) fn schedule_notice_dismiss(version: u64) -> Task<Message> {
        Task::perform(
            async move {
                tokio::time::sleep(CONNECT_NOTICE_DISMISS_DELAY).await;
                version
            },
            Message::DismissNotice,
        )
    }

    pub(super) fn active_quick_connect_launcher_for_device(
        &self,
        device_ip: &str,
    ) -> Option<&'static str> {
        if !self.is_connecting {
            return None;
        }

        self.active_quick_connect
            .as_ref()
            .and_then(|state| (state.device_ip == device_ip).then_some(state.launcher_key))
    }

    pub(super) fn quick_connect_launcher_for_tool(tool: ToolKind) -> &'static str {
        match tool {
            ToolKind::Vscode => "vscode",
            ToolKind::Mobaxterm => "mobaxterm",
            ToolKind::VncViewer => "vnc",
            ToolKind::RustDesk => "rustdesk",
        }
    }
}
