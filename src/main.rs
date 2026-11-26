// SPDX-License-Identifier: MIT

mod upower;

use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Length, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget;

use upower::Device;

const POPUP_MIN_WIDTH: f32 = 260.0;
const POPUP_MAX_WIDTH: f32 = 360.0;
const POPUP_MAX_HEIGHT: f32 = 600.0;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}

#[derive(Default)]
struct Applet {
    core: cosmic::Core,
    popup: Option<Id>,
    devices: Vec<Device>,
    panel_icon: String,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(Id),
    Devices(Vec<Device>),
}

impl cosmic::Application for Applet {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.darthsalad.CosmicAppletBatteries";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, _flags: ()) -> (Self, Task<cosmic::Action<Message>>) {
        let applet = Applet {
            core,
            panel_icon: "battery-missing-symbolic".into(),
            ..Default::default()
        };
        (applet, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(upower::watch).map(Message::Devices)
    }

    fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::Devices(devices) => {
                self.panel_icon = devices
                    .first()
                    .map_or_else(|| "battery-missing-symbolic".into(), Device::level_icon);
                self.devices = devices;
            }
            Message::TogglePopup => {
                if let Some(id) = self.popup.take() {
                    return destroy_popup(id);
                }
                let Some(parent) = self.core.main_window_id() else {
                    return Task::none();
                };
                let id = Id::unique();
                self.popup = Some(id);
                let mut settings = self
                    .core
                    .applet
                    .get_popup_settings(parent, id, None, None, None);
                settings.positioner.size_limits = Limits::NONE
                    .min_width(POPUP_MIN_WIDTH)
                    .max_width(POPUP_MAX_WIDTH)
                    .min_height(1.0)
                    .max_height(POPUP_MAX_HEIGHT);
                return get_popup(settings);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        self.core
            .applet
            .icon_button(&self.panel_icon)
            .on_press_down(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        let body: Element<'_, Message> = if self.devices.is_empty() {
            widget::text::body("No battery devices found").into()
        } else {
            self.devices
                .iter()
                .fold(widget::list_column(), |list, dev| list.add(device_row(dev)))
                .into()
        };

        let content = widget::column::with_capacity(1)
            .padding(8)
            .width(Length::Fill)
            .push(body);

        self.core.applet.popup_container(content).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn device_row(dev: &Device) -> Element<'_, Message> {
    let mut pct = format!("{:.0}%", dev.percentage);
    if dev.charging {
        pct.push_str(" ⚡");
    }

    widget::row::with_capacity(4)
        .spacing(12)
        .align_y(Alignment::Center)
        .push(widget::icon::from_name(dev.kind_icon()).size(20).symbolic(true))
        .push(widget::text::body(dev.name.as_str()).width(Length::Fill))
        .push(widget::text::body(pct))
        .push(widget::icon::from_name(dev.level_icon()).size(16).symbolic(true))
        .into()
}
