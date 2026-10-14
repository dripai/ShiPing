use crate::config::ThemePreference;
use gpui::component::{Theme, ThemeMode};
use gpui::{App, Global, Window, WindowAppearance};

impl Global for ThemePreference {}

pub(super) fn apply(preference: ThemePreference, cx: &mut App) {
    cx.set_global(preference);
    let appearance = match preference {
        ThemePreference::System => None,
        ThemePreference::Light => Some(WindowAppearance::Light),
        ThemePreference::Dark => Some(WindowAppearance::Dark),
    };
    // GPUI applies the native window override on macOS. On Windows this is a
    // no-op; the client area still uses the component theme below.
    cx.set_window_appearance(appearance);
    match preference {
        ThemePreference::System => Theme::sync_system_appearance(None, cx),
        ThemePreference::Light => Theme::change(ThemeMode::Light, None, cx),
        ThemePreference::Dark => Theme::change(ThemeMode::Dark, None, cx),
    }
}

pub(super) fn observe_system_appearance(window: &Window) {
    window
        .observe_window_appearance(|window, cx| {
            if *cx.global::<ThemePreference>() == ThemePreference::System {
                Theme::sync_system_appearance(Some(window), cx);
            }
        })
        .detach();
}
