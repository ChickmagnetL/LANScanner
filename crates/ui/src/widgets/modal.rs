use iced::alignment::{Horizontal, Vertical};
use iced::widget::{container, mouse_area, opaque, stack, text};
use iced::{Element, Fill};

use crate::theme::styles;

pub fn overlay<'a, Message>(
    base: Element<'a, Message>,
    content: impl Into<Element<'a, Message>>,
    on_backdrop: Message,
    width: f32,
) -> Element<'a, Message>
where
    Message: Clone + 'a,
{
    let backdrop = mouse_area(
        container(text(""))
            .width(Fill)
            .height(Fill)
            .style(styles::modal_backdrop),
    )
    .on_press(on_backdrop);
    let panel = container(opaque(container(content.into()).style(styles::modal_panel)))
        .width(Fill)
        .max_width(width);
    let centered_panel = container(panel)
        .width(Fill)
        .height(Fill)
        .padding(24.0)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);
    let overlay_layer = container(
        stack([backdrop.into(), centered_panel.into()])
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill);

    stack([base, overlay_layer.into()])
        .width(Fill)
        .height(Fill)
        .into()
}
