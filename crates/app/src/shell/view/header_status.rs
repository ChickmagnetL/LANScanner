use iced::widget::{container, row, text};
use iced::{Alignment, Element, Theme, border};
use ui::theme::{self, AppLanguage};

use crate::message::Message;

use super::super::{Notice, NoticeTone};

pub(super) const RESULT_HEADER_STATUS_SLOT_HEIGHT: f32 = 28.0;
pub(super) const RESULT_HEADER_FILTER_SLOT_HEIGHT: f32 = 32.0;

pub(super) fn result_header_status<'a>(
    notice: Option<&'a Notice>,
    progress_status: Option<&'a str>,
    language: AppLanguage,
) -> Option<Element<'a, Message>> {
    let mut capsules = row![].spacing(8).align_y(Alignment::Center);
    let mut has_capsule = false;

    if let Some(status) = progress_status {
        capsules = capsules.push(progress_capsule(language, compact_progress_message(status)));
        has_capsule = true;
    }

    if let Some(notice) = notice {
        capsules = capsules.push(notice_capsule(notice, language));
        has_capsule = true;
    }

    has_capsule.then(|| capsules.into())
}

fn localized(language: AppLanguage, chinese: &'static str, english: &'static str) -> &'static str {
    match language {
        AppLanguage::Chinese => chinese,
        AppLanguage::English => english,
    }
}

fn notice_capsule<'a>(notice: &'a Notice, language: AppLanguage) -> Element<'a, Message> {
    let (label, tone, background, border_color) = match notice.tone {
        NoticeTone::Success => (
            localized(language, "成功", "Success"),
            ui::theme::colors::rgb(0x16, 0xA3, 0x4A),
            ui::theme::colors::rgba(0x16, 0xA3, 0x4A, 0.10),
            ui::theme::colors::rgba(0x16, 0xA3, 0x4A, 0.24),
        ),
        NoticeTone::Warning => (
            localized(language, "提示", "Notice"),
            ui::theme::colors::rgb(0xD9, 0x77, 0x06),
            ui::theme::colors::rgba(0xF5, 0x9E, 0x0B, 0.12),
            ui::theme::colors::rgba(0xF5, 0x9E, 0x0B, 0.24),
        ),
        NoticeTone::Error => (
            localized(language, "错误", "Error"),
            ui::theme::colors::rgb(0xDC, 0x26, 0x26),
            ui::theme::colors::rgba(0xDC, 0x26, 0x26, 0.10),
            ui::theme::colors::rgba(0xDC, 0x26, 0x26, 0.24),
        ),
    };

    status_capsule(
        label,
        compact_notice_message(notice, language),
        tone,
        background,
        border_color,
    )
}

fn progress_capsule<'a>(language: AppLanguage, message: String) -> Element<'a, Message> {
    status_capsule(
        localized(language, "连接中", "Working"),
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

fn compact_notice_message(notice: &Notice, language: AppLanguage) -> String {
    let message = if notice.message.trim().is_empty() {
        match notice.tone {
            NoticeTone::Success => localized(language, "操作成功", "Operation succeeded"),
            NoticeTone::Warning => localized(language, "请留意提示", "Check the notice"),
            NoticeTone::Error => localized(language, "操作失败", "Operation failed"),
        }
    } else {
        notice.message.as_str()
    };

    truncate_header_text(message, 24)
}

fn compact_progress_message(status: &str) -> String {
    truncate_header_text(status, 24)
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
