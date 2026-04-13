use ui::credential_card::{CARD_PADDING as CREDENTIAL_CARD_PADDING, USER_DROPDOWN_TOP_OFFSET};
use ui::scan_card::{
    CARD_HEIGHT as SCAN_CARD_HEIGHT, CARD_PADDING as SCAN_CARD_PADDING,
    CONTROL_SPACING as SCAN_CARD_CONTROL_SPACING, TITLE_ROW_HEIGHT as SCAN_CARD_TITLE_ROW_HEIGHT,
};
use ui::widgets::dropdown::{MENU_GAP, TRIGGER_HEIGHT};

const TITLEBAR_HEIGHT: f32 = 54.0;
const CONTENT_TOP_PADDING: f32 = 20.0;
pub(super) const CONTENT_PADDING: f32 = 20.0;
pub(super) const LEFT_COLUMN_WIDTH: f32 = 320.0;
pub(super) const LEFT_COLUMN_SPACING: f32 = 20.0;
pub(super) const CONTENT_SPACING: f32 = 20.0;
pub(super) const RIGHT_PANEL_PADDING: f32 = 0.0;

pub(super) fn scan_dropdown_left() -> f32 {
    CONTENT_TOP_PADDING + f32::from(SCAN_CARD_PADDING)
}

pub(super) fn scan_dropdown_top() -> f32 {
    TITLEBAR_HEIGHT
        + CONTENT_TOP_PADDING
        + f32::from(SCAN_CARD_PADDING)
        + SCAN_CARD_TITLE_ROW_HEIGHT
        + f32::from(SCAN_CARD_CONTROL_SPACING)
        + TRIGGER_HEIGHT
        + MENU_GAP
}

pub(super) fn scan_dropdown_width() -> f32 {
    LEFT_COLUMN_WIDTH - (f32::from(SCAN_CARD_PADDING) * 2.0)
}

pub(super) fn credential_dropdown_left() -> f32 {
    CONTENT_TOP_PADDING + f32::from(CREDENTIAL_CARD_PADDING)
}

pub(super) fn credential_dropdown_top() -> f32 {
    TITLEBAR_HEIGHT
        + CONTENT_TOP_PADDING
        + SCAN_CARD_HEIGHT
        + LEFT_COLUMN_SPACING
        + USER_DROPDOWN_TOP_OFFSET
        + TRIGGER_HEIGHT
        + MENU_GAP
}

pub(super) fn credential_dropdown_width() -> f32 {
    LEFT_COLUMN_WIDTH - (f32::from(CREDENTIAL_CARD_PADDING) * 2.0)
}
