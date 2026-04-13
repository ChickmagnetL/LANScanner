use iced::overlay::menu;
use iced::widget::{button, container, text_input};
use iced::{Background, Border, Shadow, Theme, border};

use super::colors::{self, rgba};

const LIGHT_INPUT_BACKGROUND: iced::Color = colors::rgb(0xF4, 0xF5, 0xF7);
const LIGHT_BODY_TEXT: iced::Color = colors::rgb(0x37, 0x41, 0x51);
const LIGHT_MUTED_TEXT: iced::Color = colors::rgb(0x9C, 0xA3, 0xAF);
const WINDOW_SHELL_RADIUS_DARK: f32 = 20.0;
const WINDOW_SHELL_RADIUS_LIGHT: f32 = 20.0;

pub fn canvas(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);

    container::Style::default().background(palette.canvas)
}

pub fn window_backdrop(_theme: &Theme) -> container::Style {
    container::Style::default()
        .background(iced::Color::TRANSPARENT)
        .border(Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(0),
        })
}

pub fn window_shell_radius(theme: &Theme) -> f32 {
    let palette = colors::palette(theme);
    if palette.card == colors::DARK.card {
        WINDOW_SHELL_RADIUS_DARK
    } else {
        WINDOW_SHELL_RADIUS_LIGHT
    }
}

pub fn window_shell(theme: &Theme) -> container::Style {
    window_shell_with_radius(theme, window_shell_radius(theme))
}

pub fn window_shell_with_radius(theme: &Theme, radius: f32) -> container::Style {
    let palette = colors::palette(theme);

    container::Style::default()
        .background(palette.canvas)
        .border(Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(radius),
        })
}

pub fn titlebar(theme: &Theme) -> container::Style {
    titlebar_with_radius(theme, window_shell_radius(theme))
}

pub fn titlebar_with_radius(theme: &Theme, radius: f32) -> container::Style {
    let palette = colors::palette(theme);

    container::Style::default()
        .background(palette.titlebar)
        .border(Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(radius),
        })
}

pub fn titlebar_divider(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default().background(if is_dark {
        rgba(0x78, 0x81, 0x8F, 0.34)
    } else {
        rgba(0xD9, 0xE0, 0xE9, 0.96)
    })
}

pub fn card(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default()
        .background(if is_dark {
            palette.card
        } else {
            rgba(0xFF, 0xFF, 0xFF, 1.0)
        })
        .border(Border {
            color: palette.border,
            width: 1.0,
            radius: border::radius(16),
        })
        .shadow(Shadow::default())
}

pub fn card_panel(theme: &Theme) -> container::Style {
    card(theme)
}

pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = colors::palette(theme);
    let background = match status {
        button::Status::Pressed => rgba(0x1D, 0x4E, 0xD8, 1.0),
        button::Status::Hovered => rgba(0x25, 0x63, 0xEB, 1.0),
        button::Status::Disabled => rgba(0x3B, 0x82, 0xF6, 0.55),
        button::Status::Active => palette.primary,
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color: colors::LIGHT.card,
        border: Border {
            color: background,
            width: 1.0,
            radius: border::radius(12),
        },
        shadow: Shadow::default(),
    }
}

pub fn titlebar_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = colors::palette(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            if palette.canvas == colors::DARK.canvas {
                Background::Color(rgba(0x2A, 0x2A, 0x2A, 1.0))
            } else {
                Background::Color(rgba(0xE5, 0xE7, 0xEB, 0.92))
            }
        }
        _ => Background::Color(iced::Color::TRANSPARENT),
    };

    button::Style {
        snap: false,
        background: Some(background),
        text_color: palette.text,
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(10),
        },
        shadow: Shadow::default(),
    }
}

pub fn titlebar_tool_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.canvas == colors::DARK.canvas;
    let (background, border_color, border_width, shadow) = match status {
        button::Status::Active => (
            iced::Color::TRANSPARENT,
            iced::Color::TRANSPARENT,
            0.0,
            Shadow::default(),
        ),
        button::Status::Hovered => {
            if is_dark {
                (
                    rgba(0x2A, 0x2A, 0x2A, 0.8),
                    iced::Color::TRANSPARENT,
                    0.0,
                    Shadow::default(),
                )
            } else {
                (
                    rgba(0xE5, 0xE7, 0xEB, 0.8),
                    iced::Color::TRANSPARENT,
                    0.0,
                    Shadow::default(),
                )
            }
        }
        button::Status::Pressed => {
            if is_dark {
                (
                    rgba(0x2A, 0x2A, 0x2A, 1.0),
                    iced::Color::TRANSPARENT,
                    0.0,
                    Shadow::default(),
                )
            } else {
                (
                    rgba(0xE5, 0xE7, 0xEB, 1.0),
                    iced::Color::TRANSPARENT,
                    0.0,
                    Shadow::default(),
                )
            }
        }
        button::Status::Disabled => (
            iced::Color::TRANSPARENT,
            iced::Color::TRANSPARENT,
            0.0,
            Shadow::default(),
        ),
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color: palette.text,
        border: Border {
            color: border_color,
            width: border_width,
            radius: border::radius(11),
        },
        shadow,
    }
}

pub fn close_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = colors::palette(theme);
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.danger,
        _ => iced::Color::TRANSPARENT,
    };
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => colors::LIGHT.card,
        _ => palette.text,
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(10),
        },
        shadow: Shadow::default(),
    }
}

pub fn dropdown_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;
    let is_focused = matches!(&status, text_input::Status::Focused { .. });
    let background = match status {
        text_input::Status::Disabled => {
            if is_dark {
                iced::Color {
                    a: 0.66,
                    ..palette.input
                }
            } else {
                rgba(0xF4, 0xF5, 0xF7, 0.82)
            }
        }
        _ => {
            if is_dark {
                palette.input
            } else {
                LIGHT_INPUT_BACKGROUND
            }
        }
    };

    text_input::Style {
        background: Background::Color(background),
        border: Border {
            color: if is_focused {
                palette.primary
            } else {
                iced::Color::TRANSPARENT
            },
            width: if is_focused { 1.0 } else { 0.0 },
            radius: border::radius(12),
        },
        icon: if is_dark {
            palette.muted_text
        } else {
            LIGHT_MUTED_TEXT
        },
        placeholder: if is_dark {
            palette.muted_text
        } else {
            LIGHT_MUTED_TEXT
        },
        value: if is_dark {
            palette.text
        } else {
            LIGHT_BODY_TEXT
        },
        selection: rgba(0x3B, 0x82, 0xF6, 0.18),
    }
}

pub fn dropdown_menu(theme: &Theme) -> menu::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    menu::Style {
        background: Background::Color(if is_dark {
            palette.overlay
        } else {
            rgba(0xFF, 0xFF, 0xFF, 1.0)
        }),
        border: Border {
            color: if is_dark {
                palette.border
            } else {
                rgba(0x00, 0x00, 0x00, 0.05)
            },
            width: 1.0,
            radius: border::radius(12),
        },
        shadow: Shadow::default(),
        text_color: if is_dark {
            palette.text
        } else {
            LIGHT_BODY_TEXT
        },
        selected_text_color: if is_dark {
            palette.text
        } else {
            LIGHT_BODY_TEXT
        },
        selected_background: Background::Color(if is_dark {
            rgba(0x3B, 0x82, 0xF6, 0.14)
        } else {
            rgba(0xEF, 0xF6, 0xFF, 1.0)
        }),
    }
}

pub fn dropdown_trigger(theme: &Theme, status: button::Status, is_open: bool) -> button::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;
    let background = if is_dark {
        if is_open {
            rgba(0x3F, 0x3F, 0x46, 1.0)
        } else {
            match status {
                button::Status::Pressed => rgba(0x3F, 0x3F, 0x46, 1.0),
                button::Status::Hovered => rgba(0x3F, 0x3F, 0x46, 1.0),
                button::Status::Disabled => rgba(0x27, 0x27, 0x2A, 0.60),
                button::Status::Active => rgba(0x27, 0x27, 0x2A, 1.0),
            }
        }
    } else {
        if is_open {
            rgba(0xEA, 0xEC, 0xEF, 1.0)
        } else {
            match status {
                button::Status::Pressed => rgba(0xEA, 0xEC, 0xEF, 1.0),
                button::Status::Hovered => rgba(0xEA, 0xEC, 0xEF, 1.0),
                button::Status::Disabled => rgba(0xF4, 0xF5, 0xF7, 0.70),
                button::Status::Active => rgba(0xF4, 0xF5, 0xF7, 1.0),
            }
        }
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color: if is_dark {
            palette.text
        } else {
            LIGHT_BODY_TEXT
        },
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(12),
        },
        shadow: Shadow::default(),
    }
}

pub fn dropdown_option(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;
    let background = if is_dark {
        if is_selected {
            rgba(0x3B, 0x82, 0xF6, 0.10)
        } else {
            match status {
                button::Status::Pressed => rgba(0x3B, 0x82, 0xF6, 0.08),
                button::Status::Hovered => rgba(0x3B, 0x82, 0xF6, 0.05),
                button::Status::Disabled => iced::Color::TRANSPARENT,
                button::Status::Active => iced::Color::TRANSPARENT,
            }
        }
    } else if is_selected {
        rgba(0xEF, 0xF6, 0xFF, 1.0)
    } else {
        match status {
            button::Status::Pressed | button::Status::Hovered => rgba(0xF3, 0xF4, 0xF6, 1.0),
            button::Status::Disabled => iced::Color::TRANSPARENT,
            button::Status::Active => iced::Color::TRANSPARENT,
        }
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color: if is_dark {
            palette.text
        } else {
            LIGHT_BODY_TEXT
        },
        border: Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(8),
        },
        shadow: Shadow::default(),
    }
}

pub fn dropdown_menu_surface(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default()
        .background(if is_dark {
            palette.overlay
        } else {
            rgba(0xFF, 0xFF, 0xFF, 1.0)
        })
        .border(Border {
            color: if is_dark {
                palette.border
            } else {
                rgba(0x00, 0x00, 0x00, 0.05)
            },
            width: 1.0,
            radius: border::radius(12),
        })
        .shadow(Shadow::default())
}

pub fn dropdown_divider(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);

    container::Style::default().background(if palette.card == colors::DARK.card {
        iced::Color {
            a: 0.60,
            ..palette.border
        }
    } else {
        rgba(0x00, 0x00, 0x00, 0.06)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectButtonState {
    Available,
    Processing,
    TemporarilyLocked,
    Unavailable,
}

pub fn connect_button(
    theme: &Theme,
    status: button::Status,
    accent: iced::Color,
    state: ConnectButtonState,
) -> button::Style {
    let palette = colors::palette(theme);
    let interactive = matches!(state, ConnectButtonState::Available);
    let visually_disabled = matches!(state, ConnectButtonState::Unavailable);
    let processing = matches!(state, ConnectButtonState::Processing);
    let temporarily_locked = matches!(state, ConnectButtonState::TemporarilyLocked);

    let background = if visually_disabled {
        rgba(
            0x9C,
            0xA3,
            0xAF,
            if palette.card == colors::DARK.card {
                0.14
            } else {
                0.06
            },
        )
    } else if processing {
        rgba(
            (accent.r * 255.0).round() as u8,
            (accent.g * 255.0).round() as u8,
            (accent.b * 255.0).round() as u8,
            if palette.card == colors::DARK.card {
                0.12
            } else {
                0.045
            },
        )
    } else if temporarily_locked {
        rgba(
            (accent.r * 255.0).round() as u8,
            (accent.g * 255.0).round() as u8,
            (accent.b * 255.0).round() as u8,
            if palette.card == colors::DARK.card {
                0.08
            } else {
                0.03
            },
        )
    } else if interactive {
        match status {
            button::Status::Hovered | button::Status::Pressed => rgba(
                (accent.r * 255.0).round() as u8,
                (accent.g * 255.0).round() as u8,
                (accent.b * 255.0).round() as u8,
                if palette.card == colors::DARK.card {
                    0.16
                } else {
                    0.06
                },
            ),
            _ => palette.card,
        }
    } else {
        palette.card
    };

    let border_color = if visually_disabled {
        rgba(0x9C, 0xA3, 0xAF, 0.28)
    } else if processing {
        rgba(
            (accent.r * 255.0).round() as u8,
            (accent.g * 255.0).round() as u8,
            (accent.b * 255.0).round() as u8,
            if palette.card == colors::DARK.card {
                0.28
            } else {
                0.16
            },
        )
    } else if temporarily_locked {
        rgba(
            (accent.r * 255.0).round() as u8,
            (accent.g * 255.0).round() as u8,
            (accent.b * 255.0).round() as u8,
            if palette.card == colors::DARK.card {
                0.18
            } else {
                0.10
            },
        )
    } else if interactive {
        match status {
            button::Status::Hovered | button::Status::Pressed => rgba(
                (accent.r * 255.0).round() as u8,
                (accent.g * 255.0).round() as u8,
                (accent.b * 255.0).round() as u8,
                0.24,
            ),
            _ => palette.border,
        }
    } else {
        palette.border
    };

    button::Style {
        snap: false,
        background: Some(Background::Color(background)),
        text_color: palette.text,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: border::radius(8), // rounded-lg
        },
        shadow: Shadow::default(),
    }
}

pub fn custom_scrollbar(
    theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    let color = match status {
        iced::widget::scrollable::Status::Hovered { .. }
        | iced::widget::scrollable::Status::Dragged { .. } => {
            if is_dark {
                rgba(107, 114, 128, 0.8)
            } else {
                rgba(156, 163, 175, 0.8)
            }
        }
        _ => {
            if is_dark {
                rgba(107, 114, 128, 0.5)
            } else {
                rgba(156, 163, 175, 0.5)
            }
        }
    };

    let rail = iced::widget::scrollable::Rail {
        background: Some(Background::Color(iced::Color::TRANSPARENT)),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: border::radius(0),
        },
        scroller: iced::widget::scrollable::Scroller {
            background: Background::Color(color),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: border::radius(10),
            },
        },
    };

    iced::widget::scrollable::Style {
        container: iced::widget::container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: iced::Background::Color(iced::Color::TRANSPARENT),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: border::radius(0),
            },
            shadow: iced::Shadow::default(),
            icon: palette.muted_text,
        },
    }
}

pub fn modal_backdrop(_theme: &Theme) -> container::Style {
    container::Style::default().background(colors::MODAL_BACKDROP)
}

pub fn modal_panel(theme: &Theme) -> container::Style {
    let palette = colors::palette(theme);
    let is_dark = palette.card == colors::DARK.card;

    container::Style::default()
        .background(palette.card)
        .border(Border {
            color: if is_dark {
                rgba(0x6B, 0x74, 0x82, 0.36)
            } else {
                rgba(0xDA, 0xE0, 0xEA, 0.70)
            },
            width: 1.0,
            radius: border::radius(16),
        })
        .shadow(Shadow::default())
}
