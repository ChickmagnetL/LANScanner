use std::time::Duration;

use iced::{Subscription, window};

use crate::message::Message;

use super::ShellApp;

impl ShellApp {
    pub fn subscription(&self) -> Subscription<Message> {
        let animation = if self.is_scanning || self.is_refreshing_networks || self.is_verifying {
            iced::time::every(Duration::from_millis(80)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        let visual_check_tick = if self.visual_check.is_some() {
            iced::time::every(Duration::from_millis(120)).map(|_| Message::VisualCheckFrameTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            animation,
            visual_check_tick,
            window::open_events().map(Message::WindowReady),
            window::resize_events().map(|(window_id, _)| Message::WindowResized(window_id)),
        ])
    }
}
