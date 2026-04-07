use iced::widget::{Space, column, mouse_area, row};
use iced::{Element, Fill, Length, mouse, window};

use crate::message::Message;

const WINDOW_RESIZE_HANDLE_THICKNESS: f32 = 6.0;
const WINDOW_RESIZE_CORNER_SPAN: f32 = 14.0;

pub(super) fn window_resize_overlay() -> Element<'static, Message> {
    if !platform::window::uses_custom_resize_overlay() {
        return Space::new().width(Fill).height(Fill).into();
    }

    column![
        row![
            window_resize_handle(
                window::Direction::NorthWest,
                Length::Fixed(WINDOW_RESIZE_CORNER_SPAN),
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingDiagonallyDown,
            ),
            window_resize_handle(
                window::Direction::North,
                Fill,
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            ),
            window_resize_handle(
                window::Direction::NorthEast,
                Length::Fixed(WINDOW_RESIZE_CORNER_SPAN),
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingDiagonallyUp,
            ),
        ]
        .width(Fill)
        .height(Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS)),
        row![
            window_resize_handle(
                window::Direction::West,
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                Fill,
                mouse::Interaction::ResizingHorizontally,
            ),
            Space::new().width(Fill).height(Fill),
            window_resize_handle(
                window::Direction::East,
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                Fill,
                mouse::Interaction::ResizingHorizontally,
            ),
        ]
        .width(Fill)
        .height(Fill),
        row![
            window_resize_handle(
                window::Direction::SouthWest,
                Length::Fixed(WINDOW_RESIZE_CORNER_SPAN),
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingDiagonallyUp,
            ),
            window_resize_handle(
                window::Direction::South,
                Fill,
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingVertically,
            ),
            window_resize_handle(
                window::Direction::SouthEast,
                Length::Fixed(WINDOW_RESIZE_CORNER_SPAN),
                Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS),
                mouse::Interaction::ResizingDiagonallyDown,
            ),
        ]
        .width(Fill)
        .height(Length::Fixed(WINDOW_RESIZE_HANDLE_THICKNESS)),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

fn window_resize_handle(
    direction: window::Direction,
    width: Length,
    height: Length,
    interaction: mouse::Interaction,
) -> Element<'static, Message> {
    mouse_area(Space::new().width(width).height(height))
        .on_press(Message::WindowAction(
            platform::window::WindowAction::Resize(direction),
        ))
        .interaction(interaction)
        .into()
}
