use crate::i18n::text;
use anyhow::{Context, Result};
use std::cell::Cell;
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

pub(super) struct RecordingTray {
    _icon: TrayIcon,
    items: Vec<MenuItem>,
    last_state: Cell<Option<(bool, bool, bool)>>,
}

impl RecordingTray {
    pub(super) fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        gtk::init().context("initialize GTK for tray-icon")?;
        let image = image::load_from_memory(include_bytes!("../../assets/app.png"))?.into_rgba8();
        let (width, height) = image.dimensions();
        let icon = tray_icon::Icon::from_rgba(image.into_raw(), width, height)?;
        let menu = Menu::new();
        let items = labels()
            .iter()
            .map(|label| MenuItem::new(*label, true, None))
            .collect::<Vec<_>>();
        for item in &items {
            menu.append(item)?;
        }
        let icon = TrayIconBuilder::new()
            .with_tooltip("ShiPing")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .context("create ShiPing tray")?;
        Ok(Self {
            _icon: icon,
            items,
            last_state: Cell::new(None),
        })
    }

    pub(super) fn refresh(&self, busy: bool, can_pause: bool) {
        let state = (text("zh", "en") == "en", busy, can_pause);
        if self.last_state.replace(Some(state)) == Some(state) {
            return;
        }
        for (item, label) in self.items.iter().zip(labels()) {
            item.set_text(label);
        }
        self.items[1].set_enabled(!busy);
        self.items[2].set_enabled(can_pause);
        self.items[3].set_enabled(busy);
        self.items[4].set_enabled(!busy);
    }

    pub(super) fn drain_actions(&self) -> Vec<usize> {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
        let mut actions = MenuEvent::receiver()
            .try_iter()
            .filter_map(|event| self.items.iter().position(|item| item.id() == &event.id))
            .collect::<Vec<_>>();
        for event in TrayIconEvent::receiver().try_iter() {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                actions.push(0);
            }
        }
        actions
    }
}

fn labels() -> [&'static str; 6] {
    [
        text("显示拾屏", "Show ShiPing"),
        text("开始录制", "Start recording"),
        text("暂停 / 继续", "Pause / Resume"),
        text("停止录制", "Stop recording"),
        text("设置", "Preferences"),
        text("退出", "Quit"),
    ]
}
