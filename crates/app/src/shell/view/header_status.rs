use iced::widget::{container, row, text};
use iced::{Alignment, Element, Theme, border};
use ui::theme;

use crate::message::Message;

use super::super::{Notice, NoticeTone};

pub(super) const RESULT_HEADER_STATUS_SLOT_HEIGHT: f32 = 28.0;
pub(super) const RESULT_HEADER_FILTER_SLOT_HEIGHT: f32 = 32.0;

pub(super) fn result_header_status<'a>(
    notice: Option<&'a Notice>,
    progress_status: Option<&'a str>,
) -> Option<Element<'a, Message>> {
    let mut capsules = row![].spacing(8).align_y(Alignment::Center);
    let mut has_capsule = false;

    if let Some(status) = progress_status {
        capsules = capsules.push(progress_capsule(compact_progress_message(status)));
        has_capsule = true;
    }

    if let Some(notice) = notice {
        capsules = capsules.push(notice_capsule(notice));
        has_capsule = true;
    }

    has_capsule.then(|| capsules.into())
}

fn notice_capsule<'a>(notice: &'a Notice) -> Element<'a, Message> {
    let (label, tone, background, border_color) = match notice.tone {
        NoticeTone::Success => (
            "成功",
            ui::theme::colors::rgb(0x16, 0xA3, 0x4A),
            ui::theme::colors::rgba(0x16, 0xA3, 0x4A, 0.10),
            ui::theme::colors::rgba(0x16, 0xA3, 0x4A, 0.24),
        ),
        NoticeTone::Warning => (
            "提示",
            ui::theme::colors::rgb(0xD9, 0x77, 0x06),
            ui::theme::colors::rgba(0xF5, 0x9E, 0x0B, 0.12),
            ui::theme::colors::rgba(0xF5, 0x9E, 0x0B, 0.24),
        ),
        NoticeTone::Error => (
            "错误",
            ui::theme::colors::rgb(0xDC, 0x26, 0x26),
            ui::theme::colors::rgba(0xDC, 0x26, 0x26, 0.10),
            ui::theme::colors::rgba(0xDC, 0x26, 0x26, 0.24),
        ),
    };

    status_capsule(
        label,
        compact_notice_message(notice),
        tone,
        background,
        border_color,
    )
}

fn progress_capsule<'a>(message: String) -> Element<'a, Message> {
    status_capsule(
        "连接中",
        message,
        ui::theme::colors::BRAND_BLUE,
        ui::theme::colors::rgba(0x3B, 0x82, 0xF6, 0.08),
        ui::theme::colors::rgba(0x3B, 0x82, 0xF6, 0.20),
    )
}

fn status_capsule<'a>(
    label: &'static str,
    message: String,
    tone: iced::Color,
    background: iced::Color,
    border_color: iced::Color,
) -> Element<'a, Message> {
    container(
        row![
            text(label)
                .font(ui::theme::fonts::body())
                .size(11)
                .style(move |_| theme::solid_text(tone)),
            text(message)
                .font(ui::theme::fonts::body())
                .size(11)
                .style(|theme: &Theme| theme::text_primary(theme)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(move |_| {
        container::Style::default()
            .background(background)
            .border(iced::Border {
                color: border_color,
                width: 1.0,
                radius: border::radius(999),
            })
    })
    .into()
}

fn compact_notice_message(notice: &Notice) -> String {
    let message = notice.message.as_str();
    let compact = match notice.tone {
        NoticeTone::Success => {
            if message.contains("免密") {
                "免密准备完成"
            } else if message.contains("已启动") {
                "连接已启动"
            } else {
                "操作成功"
            }
        }
        NoticeTone::Warning => {
            if message.contains("自动检测SSH凭证中") {
                "自动验证中"
            } else if message.contains("暂未开放") {
                "功能暂未开放"
            } else if message.contains("用户名") {
                "请填写 SSH 用户名"
            } else if message.contains("密码") {
                "需手动输入密码"
            } else if message.contains("认证") {
                "认证需人工确认"
            } else {
                "请留意提示"
            }
        }
        NoticeTone::Error => {
            if message.contains("路径") {
                "工具路径配置失败"
            } else if message.contains("Docker") {
                "Docker 操作失败"
            } else if message.contains("启动") {
                "连接启动失败"
            } else if message.contains("认证") || message.contains("SSH") {
                "SSH 认证失败"
            } else {
                "操作失败"
            }
        }
    };

    truncate_header_text(compact, 16)
}

fn compact_progress_message(status: &str) -> String {
    let compact = if status.contains("Docker") && status.contains("容器") {
        "准备 Docker 容器"
    } else if status.contains("Docker") {
        "处理 Docker"
    } else if status.contains("终端") {
        "启动终端连接"
    } else if status.contains("VS Code") {
        "启动 VS Code"
    } else if status.contains("MobaXterm") {
        "启动 MobaXterm"
    } else if status.contains("VNC") {
        "启动 VNC"
    } else if status.contains("连接") {
        "启动连接"
    } else {
        "处理中"
    };

    truncate_header_text(compact, 16)
}

fn truncate_header_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_owned();
    }

    let keep = max_chars.saturating_sub(3);
    let mut output: String = input.chars().take(keep).collect();
    output.push_str("...");
    output
}
