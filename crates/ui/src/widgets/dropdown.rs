use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text};
use iced::{Alignment, Element, Fill, Length, Padding, Theme, border};

use crate::theme::{
    self, colors, fonts,
    icons::{self, Glyph},
};

pub const TRIGGER_HEIGHT: f32 = 46.0;
pub const MENU_GAP: f32 = 8.0;
pub const MENU_MAX_HEIGHT: f32 = 196.0;
const MENU_ITEM_ESTIMATED_HEIGHT: f32 = 50.0;
const MENU_VERTICAL_PADDING: f32 = 8.0;
const MENU_SURFACE_PADDING: f32 = 6.0;
const MENU_SECTION_HEIGHT: f32 = 31.0;
const MENU_DIVIDER_HEIGHT: f32 = 1.0;
const MIN_CREDENTIAL_LIST_HEIGHT: f32 = 40.0;
const CHEVRON_SLOT: f32 = icons::DROPDOWN_CHEVRON_SLOT;
const CHEVRON_GLYPH: f32 = icons::DROPDOWN_CHEVRON_GLYPH;
const TRIGGER_ICON_SLOT: f32 = 22.0;
const TRIGGER_ICON_GLYPH: f32 = 17.5;
const OPTION_ICON_SLOT: f32 = 20.0;
const OPTION_ICON_GLYPH: f32 = 15.5;
const OPTION_ICON_EMPHASIZED_SLOT: f32 = 26.0;
const OPTION_ICON_EMPHASIZED_GLYPH: f32 = 20.0;
const LIGHT_VALUE_TEXT: iced::Color = colors::rgb(0x37, 0x41, 0x51);
const LIGHT_MUTED_TEXT: iced::Color = colors::rgb(0x9C, 0xA3, 0xAF);
const LIGHT_INACTIVE_ICON: iced::Color = colors::rgb(0x4B, 0x55, 0x63);

#[derive(Debug, Clone, Copy)]
pub struct DropdownPlacement {
    pub left: f32,
    pub top: f32,
    pub width: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct DropdownPlaceholder<'a> {
    pub glyph: Glyph,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct DropdownState<Message> {
    pub is_open: bool,
    pub on_toggle: Option<Message>,
    pub on_dismiss: Message,
}

#[derive(Debug, Clone)]
pub struct DropdownEntry {
    pub glyph: Glyph,
    pub title: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct OverlayChrome<'a, Message> {
    placeholder: DropdownPlaceholder<'a>,
    footer_action: Option<Message>,
}

pub struct DropdownProps<'a, T, Message, Describe, Select>
where
    T: Clone + PartialEq + 'a,
    Message: Clone + 'a,
    Describe: Fn(&T) -> DropdownEntry + Copy + 'a,
    Select: Fn(T) -> Message + Copy + 'a,
{
    pub items: &'a [T],
    pub selection: Option<&'a T>,
    pub placeholder: DropdownPlaceholder<'a>,
    pub show_trigger_icon: bool,
    pub show_option_icon: bool,
    pub footer_action: Option<Message>,
    pub state: DropdownState<Message>,
    pub placement: DropdownPlacement,
    pub describe: Describe,
    pub on_selected: Select,
}

pub fn render<'a, T, Message>(
    props: DropdownProps<
        'a,
        T,
        Message,
        impl Fn(&T) -> DropdownEntry + Copy + 'a,
        impl Fn(T) -> Message + Copy + 'a,
    >,
    content: impl FnOnce(Element<'a, Message>) -> Element<'a, Message>,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    let DropdownProps {
        items,
        selection,
        placeholder,
        show_trigger_icon,
        show_option_icon,
        footer_action,
        state,
        placement,
        describe,
        on_selected,
    } = props;

    let trigger = trigger(
        selection,
        placeholder,
        show_trigger_icon,
        state.is_open,
        state.on_toggle.clone(),
        describe,
    );
    let base = content(trigger);

    if state.is_open {
        let backdrop =
            mouse_area(container(text("")).width(Fill).height(Fill)).on_press(state.on_dismiss);

        stack([
            base,
            backdrop.into(),
            overlay(
                items,
                selection,
                placement,
                show_option_icon,
                OverlayChrome {
                    placeholder,
                    footer_action,
                },
                describe,
                on_selected,
            ),
        ])
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        base
    }
}

fn trigger<'a, T, Message>(
    selection: Option<&'a T>,
    placeholder: DropdownPlaceholder<'a>,
    show_trigger_icon: bool,
    is_open: bool,
    on_toggle: Option<Message>,
    describe: impl Fn(&T) -> DropdownEntry + Copy + 'a,
) -> Element<'a, Message>
where
    T: 'a,
    Message: Clone + 'a,
{
    let selected = selection.map(describe);
    let is_disabled = on_toggle.is_none();
    let mut trigger = button(trigger_content(
        selected.as_ref(),
        placeholder,
        show_trigger_icon,
        is_open,
        is_disabled,
    ))
    .width(Fill)
    .padding([10, 14])
    .style(move |theme: &Theme, status| theme::styles::dropdown_trigger(theme, status, is_open));

    if let Some(message) = on_toggle {
        trigger = trigger.on_press(message);
    }

    container(trigger)
        .width(Fill)
        .height(Length::Fixed(TRIGGER_HEIGHT))
        .into()
}

fn overlay<'a, T, Message>(
    items: &'a [T],
    selection: Option<&'a T>,
    placement: DropdownPlacement,
    show_option_icon: bool,
    chrome: OverlayChrome<'a, Message>,
    describe: impl Fn(&T) -> DropdownEntry + Copy + 'a,
    on_selected: impl Fn(T) -> Message + Copy + 'a,
) -> Element<'a, Message>
where
    T: Clone + PartialEq + 'a,
    Message: Clone + 'a,
{
    let item_count = items.len().max(1);
    let list_content_height =
        item_count as f32 * MENU_ITEM_ESTIMATED_HEIGHT + MENU_VERTICAL_PADDING;

    let list = items.iter().fold(column!().spacing(2), |column, item| {
        let entry = describe(item);
        let is_selected = selection == Some(item);

        let option = button(option_content(&entry, show_option_icon))
            .width(Fill)
            .padding([12, 14])
            .style(move |theme: &Theme, status| {
                theme::styles::dropdown_option(theme, status, is_selected)
            })
            .on_press(on_selected(item.clone()));

        column.push(option)
    });

    let menu_body: Element<'a, Message> = if show_option_icon {
        let menu_height = list_content_height.min(MENU_MAX_HEIGHT);
        scrollable(list)
            .width(Length::Fixed(placement.width))
            .height(Length::Fixed(menu_height))
            .style(menu_scroll_style)
            .into()
    } else {
        let OverlayChrome {
            placeholder,
            footer_action,
        } = chrome;
        if let Some(action) = footer_action {
            let header = container(text(placeholder.title).font(fonts::body()).size(11).style(
                |theme: &Theme| {
                    let palette = colors::palette(theme);
                    if palette.card == colors::DARK.card {
                        theme::text_muted(theme)
                    } else {
                        theme::solid_text(LIGHT_MUTED_TEXT)
                    }
                },
            ))
            .width(Fill)
            .padding([10, 14]);
            let footer = button(
                row![
                    icons::centered(Glyph::Plus, 18.0, 11.5, colors::BRAND_BLUE),
                    text(footer_label(placeholder))
                        .font(fonts::body())
                        .size(11)
                        .style(|_| theme::solid_text(colors::BRAND_BLUE)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([10, 14])
            .style(move |_theme: &Theme, status| {
                let (background, border_color) = match status {
                    button::Status::Hovered | button::Status::Pressed => (
                        colors::rgba(0x3B, 0x82, 0xF6, 0.05),
                        colors::rgba(0x3B, 0x82, 0xF6, 0.12),
                    ),
                    _ => (iced::Color::TRANSPARENT, iced::Color::TRANSPARENT),
                };

                button::Style {
                    snap: false,
                    background: Some(iced::Background::Color(background)),
                    text_color: colors::BRAND_BLUE,
                    border: iced::Border {
                        color: border_color,
                        width: 1.0,
                        radius: border::radius(10),
                    },
                    shadow: iced::Shadow::default(),
                }
            })
            .on_press(action);
            let chrome_height = MENU_SECTION_HEIGHT
                + MENU_DIVIDER_HEIGHT
                + MENU_SURFACE_PADDING * 2.0
                + MENU_SECTION_HEIGHT
                + MENU_DIVIDER_HEIGHT;
            let menu_height = (list_content_height + chrome_height).min(MENU_MAX_HEIGHT);
            let list_height = (menu_height - chrome_height).max(MIN_CREDENTIAL_LIST_HEIGHT);
            column![
                header,
                divider_line(),
                scrollable(list)
                    .width(Length::Fixed(placement.width))
                    .height(Length::Fixed(list_height))
                    .style(menu_scroll_style),
                divider_line(),
                footer,
            ]
            .spacing(0)
            .into()
        } else {
            let menu_height = list_content_height.min(MENU_MAX_HEIGHT);
            scrollable(list)
                .width(Length::Fixed(placement.width))
                .height(Length::Fixed(menu_height))
                .style(menu_scroll_style)
                .into()
        }
    };

    let menu = container(menu_body)
        .width(Length::Fixed(placement.width))
        .padding(MENU_SURFACE_PADDING)
        .style(theme::styles::dropdown_menu_surface)
        .clip(true);

    container(menu)
        .width(Fill)
        .height(Fill)
        .padding(Padding {
            top: placement.top,
            right: 0.0,
            bottom: 0.0,
            left: placement.left,
        })
        .align_x(Horizontal::Left)
        .align_y(Vertical::Top)
        .into()
}

fn trigger_content<'a, Message>(
    selected: Option<&DropdownEntry>,
    placeholder: DropdownPlaceholder<'_>,
    show_trigger_icon: bool,
    is_open: bool,
    is_disabled: bool,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let glyph = selected.map_or(placeholder.glyph, |entry| entry.glyph);
    let title = selected.map_or_else(|| placeholder.title.to_owned(), |entry| entry.title.clone());
    let details = selected
        .map(|entry| entry.details.clone())
        .unwrap_or_else(|| {
            placeholder
                .subtitle
                .map(|subtitle| vec![subtitle.to_owned()])
                .unwrap_or_default()
        });
    let has_selection = selected.is_some();

    if show_trigger_icon {
        row![
            trigger_leading_icon(glyph, is_disabled),
            container(trigger_copy::<Message>(
                title,
                details,
                has_selection,
                is_disabled,
                true
            ))
            .width(Fill)
            .clip(true),
            chevron(is_open, is_disabled),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    } else {
        row![
            container(trigger_copy::<Message>(
                title,
                details,
                has_selection,
                is_disabled,
                false
            ))
            .width(Fill),
            chevron(is_open, is_disabled),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }
}

fn option_content<'a, Message>(
    entry: &DropdownEntry,
    show_option_icon: bool,
) -> Element<'a, Message>
where
    Message: 'a,
{
    if show_option_icon {
        row![
            option_leading_icon(entry.glyph, false),
            container(option_copy::<Message>(
                entry.title.clone(),
                entry.details.clone()
            ))
            .width(Fill),
        ]
        .spacing(9)
        .align_y(Alignment::Center)
        .into()
    } else {
        container(option_copy::<Message>(
            entry.title.clone(),
            entry.details.clone(),
        ))
        .width(Fill)
        .into()
    }
}

fn trigger_copy<'a, Message>(
    title: String,
    details: Vec<String>,
    emphasize: bool,
    is_disabled: bool,
    compact: bool,
) -> Element<'a, Message>
where
    Message: 'a,
{
    let display_title = if compact {
        summarize_trigger_title(&title, &details).unwrap_or_else(|| title.clone())
    } else {
        title.clone()
    };

    let title = text(display_title)
        .font(fonts::semibold())
        .size(14)
        .width(Fill)
        .wrapping(iced::widget::text::Wrapping::None)
        .style(move |theme: &Theme| {
            let palette = colors::palette(theme);
            let mut color = if emphasize {
                if palette.card == colors::DARK.card {
                    palette.text
                } else {
                    LIGHT_VALUE_TEXT
                }
            } else {
                if palette.card == colors::DARK.card {
                    palette.muted_text
                } else {
                    LIGHT_MUTED_TEXT
                }
            };
            if is_disabled {
                color.a = if palette.card == colors::DARK.card {
                    0.58
                } else {
                    0.48
                };
            }
            theme::solid_text(color)
        });

    let column = column![container(title).width(Fill).clip(compact)];

    let detail_text = if details.is_empty() || compact {
        None
    } else {
        Some(details.join("  ·  "))
    };

    if let Some(detail) = detail_text {
        column
            .push(
                text(detail)
                    .font(fonts::body())
                    .size(11)
                    .style(move |theme: &Theme| {
                        let palette = colors::palette(theme);
                        let mut color = if palette.card == colors::DARK.card {
                            palette.muted_text
                        } else {
                            LIGHT_MUTED_TEXT
                        };
                        if is_disabled {
                            color.a = if palette.card == colors::DARK.card {
                                0.52
                            } else {
                                0.42
                            };
                        }
                        theme::solid_text(color)
                    }),
            )
            .spacing(3)
            .into()
    } else {
        column.into()
    }
}

fn option_copy<'a, Message>(title: String, details: Vec<String>) -> Element<'a, Message>
where
    Message: 'a,
{
    let title = text(title)
        .font(fonts::semibold())
        .size(13)
        .width(Fill)
        .wrapping(iced::widget::text::Wrapping::None)
        .style(|theme: &Theme| {
            let palette = colors::palette(theme);
            if palette.card == colors::DARK.card {
                theme::text_primary(theme)
            } else {
                theme::solid_text(LIGHT_VALUE_TEXT)
            }
        });

    if details.is_empty() {
        return column![container(title).width(Fill).clip(true)].into();
    }

    let detail_lines = option_visible_details(&details).into_iter().take(2).fold(
        column!().spacing(1),
        |column, detail| {
            column.push(
                text(detail)
                    .font(fonts::body())
                    .size(10)
                    .style(|theme: &Theme| {
                        let palette = colors::palette(theme);
                        if palette.card == colors::DARK.card {
                            theme::text_muted(theme)
                        } else {
                            theme::solid_text(LIGHT_MUTED_TEXT)
                        }
                    }),
            )
        },
    );

    column![container(title).width(Fill).clip(true), detail_lines]
        .spacing(4)
        .into()
}

fn footer_label(placeholder: DropdownPlaceholder<'_>) -> String {
    let title = placeholder.title.trim();
    let subject = dropdown_subject(title);

    if subject.is_empty() {
        if prefers_english_copy(title) {
            String::from("Add Option...")
        } else {
            String::from("添加选项...")
        }
    } else if prefers_english_copy(title) {
        format!("Add {}...", subject)
    } else {
        format!("添加{}...", subject)
    }
}

fn summarize_trigger_title(title: &str, details: &[String]) -> Option<String> {
    details
        .iter()
        .find(|detail| detail_kind(detail) == Some(DetailKind::Interface))
        .or_else(|| {
            details
                .iter()
                .find(|detail| detail_kind(detail) == Some(DetailKind::Subnet))
        })
        .or_else(|| details.first())
        .map(|detail| format!("{title} ({})", strip_detail_prefix(detail)))
}

fn option_visible_details(details: &[String]) -> Vec<String> {
    let kind = details
        .iter()
        .find(|detail| detail_kind(detail).is_none())
        .map(|detail| detail.trim().to_owned());
    let subnet = details
        .iter()
        .find(|detail| detail_kind(detail) == Some(DetailKind::Subnet))
        .map(|detail| strip_detail_prefix(detail));
    let iface = details
        .iter()
        .find(|detail| detail_kind(detail) == Some(DetailKind::Interface))
        .map(|detail| strip_detail_prefix(detail));

    if subnet.is_some() || kind.is_some() {
        let mut visible = Vec::new();

        if let Some(kind) = kind {
            visible.push(kind);
        }

        if let Some(subnet) = subnet {
            visible.push(subnet);
        }

        if visible.len() < 2 {
            if let Some(iface) = iface {
                visible.push(iface);
            }
        }

        return visible;
    }

    let mut visible = Vec::new();

    if visible.is_empty() {
        visible.extend(
            details
                .iter()
                .take(2)
                .map(|detail| detail.trim().to_owned()),
        );
    } else if visible.len() == 1 && details.len() > 1 {
        if let Some(kind_or_hint) = details.iter().find(|detail| detail_kind(detail).is_none()) {
            visible.push(kind_or_hint.trim().to_owned());
        }
    }

    visible
}

fn strip_detail_prefix(detail: &str) -> String {
    detail_body(detail).trim().to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailKind {
    Subnet,
    Interface,
}

fn detail_kind(detail: &str) -> Option<DetailKind> {
    let trimmed = detail.trim();

    if starts_with_any(trimmed, &["网段 ", "subnet "]) {
        Some(DetailKind::Subnet)
    } else if starts_with_any(trimmed, &["接口 ", "interface "]) {
        Some(DetailKind::Interface)
    } else {
        None
    }
}

fn detail_body(detail: &str) -> &str {
    trim_prefix_case_insensitive(detail.trim(), &["网段 ", "接口 ", "Subnet ", "Interface "])
}

fn dropdown_subject(title: &str) -> &str {
    let without_ellipsis = title.trim_end_matches("...");
    trim_prefix_case_insensitive(without_ellipsis.trim(), &["请选择", "选择", "Select "]).trim()
}

fn prefers_english_copy(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn starts_with_any(text: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| {
        text.to_ascii_lowercase()
            .starts_with(&prefix.to_ascii_lowercase())
    })
}

fn trim_prefix_case_insensitive<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
    let lower = text.to_ascii_lowercase();

    for prefix in prefixes {
        let lower_prefix = prefix.to_ascii_lowercase();
        if lower.starts_with(&lower_prefix) {
            return text[prefix.len()..].trim_start();
        }
    }

    text
}

fn is_dark_surface(theme: &Theme) -> bool {
    colors::palette(theme).card == colors::DARK.card
}

fn icon_tone(theme: &Theme) -> iced::Color {
    if is_dark_surface(theme) {
        colors::palette(theme).text
    } else {
        LIGHT_INACTIVE_ICON
    }
}

fn trigger_icon_disabled_tone(theme: &Theme) -> iced::Color {
    if is_dark_surface(theme) {
        iced::Color {
            a: 0.52,
            ..colors::palette(theme).text
        }
    } else {
        colors::rgba(0x9C, 0xA3, 0xAF, 0.56)
    }
}

fn option_icon_disabled_tone(theme: &Theme) -> iced::Color {
    if is_dark_surface(theme) {
        iced::Color {
            a: 0.48,
            ..colors::palette(theme).text
        }
    } else {
        colors::rgba(0x9C, 0xA3, 0xAF, 0.44)
    }
}

fn trigger_leading_icon<'a, Message>(glyph: Glyph, is_disabled: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    container(icons::themed_centered(
        glyph,
        TRIGGER_ICON_SLOT,
        TRIGGER_ICON_GLYPH,
        if is_disabled {
            trigger_icon_disabled_tone
        } else {
            icon_tone
        },
    ))
    .width(Length::Fixed(TRIGGER_ICON_SLOT))
    .height(Length::Fixed(TRIGGER_ICON_SLOT))
    .into()
}

fn option_leading_icon<'a, Message>(glyph: Glyph, is_disabled: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    let (slot, glyph_size) = option_icon_metrics(glyph);
    icons::themed_centered(
        glyph,
        slot,
        glyph_size,
        if is_disabled {
            option_icon_disabled_tone
        } else {
            icon_tone
        },
    )
}

fn option_icon_metrics(glyph: Glyph) -> (f32, f32) {
    match glyph {
        Glyph::Ethernet | Glyph::Docker => {
            (OPTION_ICON_EMPHASIZED_SLOT, OPTION_ICON_EMPHASIZED_GLYPH)
        }
        _ => (OPTION_ICON_SLOT, OPTION_ICON_GLYPH),
    }
}

fn chevron<'a, Message>(is_open: bool, is_disabled: bool) -> Element<'a, Message>
where
    Message: 'a,
{
    let glyph = if is_open {
        Glyph::ChevronUp
    } else {
        Glyph::ChevronDown
    };

    icons::themed_centered(
        glyph,
        CHEVRON_SLOT,
        CHEVRON_GLYPH,
        if is_disabled {
            trigger_icon_disabled_tone
        } else {
            icon_tone
        },
    )
}

fn divider_line<'a, Message: 'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Fill)
        .height(1)
        .style(theme::styles::dropdown_divider)
        .into()
}

fn menu_scroll_style(
    theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    theme::styles::custom_scrollbar(theme, status)
}
