use super::{Choice, Phase, RecorderView};
use crate::{
    config::Config,
    domain::{Bounds as CaptureBounds, RecordingTarget},
    i18n::text,
    platform::{desktop_integration, recording_backend},
    ui::hotkeys::display_shortcut,
};
use gpui::{
    assets::IconName,
    component::{
        ActiveTheme, Disableable, Icon, Sizable,
        button::{Button, ButtonVariants, Toggle, ToggleVariants},
        h_flex,
        native_menu::NativeMenu,
        v_flex,
    },
    prelude::FluentBuilder as _,
    *,
};

#[derive(Clone, Debug, PartialEq, Action)]
#[action(namespace = shiping, no_json)]
enum ToolbarAction {
    Choose { choice: Choice, index: usize },
    Toggle { index: usize, value: bool },
    Preferences,
    OutputDirectory,
    RefreshScreens,
    Minimize,
    Quit,
}

fn icon(name: IconName) -> Icon {
    Icon::new(name).size_4()
}

fn toolbar_button(id: &'static str) -> Button {
    Button::new(id)
        .with_size(px(34.))
        .size(px(34.))
        .rounded(px(17.))
}

fn toolbar_cell(control: impl IntoElement, label: &'static str, cx: &App) -> Div {
    v_flex()
        .w(px(40.))
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .child(control)
        .child(
            div()
                .text_size(px(11.))
                .line_height(px(14.))
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
}

fn drag_surface() -> Div {
    div()
        .window_control_area(WindowControlArea::Drag)
        .when(cfg!(target_os = "macos"), |this| {
            // Follow GPUI's window_shadow example: let the platform own the drag.
            this.on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
        })
}

fn choice_menu(
    label: impl Into<SharedString>,
    choice: Choice,
    selected: usize,
    values: &[String],
    enabled: bool,
) -> Menu {
    Menu::new(label)
        .disabled(!enabled)
        .items(values.iter().enumerate().map(|(index, value)| {
            MenuItem::action(value.clone(), ToolbarAction::Choose { choice, index })
                .checked(index == selected)
                .disabled(!enabled)
        }))
}

fn toggle_item(index: usize, label: &'static str, checked: bool, enabled: bool) -> MenuItem {
    MenuItem::action(
        label,
        ToolbarAction::Toggle {
            index,
            value: !checked,
        },
    )
    .checked(checked)
    .disabled(!enabled)
}

fn target_for_source(
    source: u8,
    screen: Option<CaptureBounds>,
    selected: Option<RecordingTarget>,
) -> Option<RecordingTarget> {
    match source {
        0 => screen.map(RecordingTarget::Screen),
        1 | 2 => selected.filter(|target| {
            matches!(
                (source, target),
                (1, RecordingTarget::Window { .. }) | (2, RecordingTarget::Region(_))
            )
        }),
        _ => None,
    }
}

impl RecorderView {
    pub(super) fn selected_target(&self) -> Option<RecordingTarget> {
        target_for_source(
            self.state.config.source_mode,
            self.selected_screen,
            self.state.target,
        )
    }

    fn target_description(&self) -> (String, String, IconName) {
        let Some(target) = self.selected_target() else {
            return (
                match self.state.config.source_mode {
                    0 => text("未选择显示器", "No display selected"),
                    1 => text("未选择窗口", "No window selected"),
                    _ => text("未选择区域", "No region selected"),
                }
                .into(),
                String::new(),
                source_icon(self.state.config.source_mode),
            );
        };
        let bounds = self.indicator_capture.unwrap_or(target.initial_bounds());
        let title = match target {
            RecordingTarget::Screen(_) => {
                let index = self
                    .monitors
                    .as_ref()
                    .and_then(|monitors| monitors.index_of(bounds));
                match index {
                    Some(index) => format!("{} {}", text("显示器", "Display"), index + 1),
                    None => text("已选屏幕", "Selected display").into(),
                }
            }
            RecordingTarget::Window { .. } => self
                .selected_window_title
                .as_deref()
                .filter(|title| !title.is_empty())
                .unwrap_or(text("已选窗口", "Selected window"))
                .to_owned(),
            RecordingTarget::Region(_) => text("已选区域", "Selected region").into(),
        };
        (
            title,
            format!("{} × {}", bounds.width, bounds.height),
            source_icon(self.state.config.source_mode),
        )
    }

    fn option_button(
        &self,
        id: &'static str,
        label: &'static str,
        choice: Choice,
        selected: usize,
        values: Vec<String>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let display = match choice {
            Choice::Quality => [text("自动", "Auto"), "720", "1080", "1:1"][selected].to_owned(),
            Choice::FrameRate => self
                .state
                .config
                .output_format
                .frames_per_second(self.state.config.frame_rate)
                .to_string(),
            _ => unreachable!("toolbar option is quality or frame rate"),
        };
        toolbar_button(id)
            .accessibility_label(format!("{label}: {}", values[selected]))
            .tooltip(format!("{label}: {}", values[selected]))
            .child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(display),
            )
            .disabled(self.busy() || self.preferences.is_some())
            .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                let menu = choice_menu(label, choice, selected, &values, true);
                view.show_native_menu(menu, event.position(), window, cx);
            }))
            .into_any_element()
    }

    fn open_save_directory(&mut self, cx: &mut Context<Self>) {
        let path = &self.state.config.save_directory;
        let result = std::fs::create_dir_all(path)
            .map_err(anyhow::Error::from)
            .and_then(|_| desktop_integration().open_path(path));
        if let Err(error) = result {
            self.fail(error);
        }
        cx.notify();
    }

    fn show_native_menu(
        &self,
        menu: Menu,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // NativeMenu dispatches Actions to the focused element. Keep the toolbar
        // in the dispatch path even when it was opened from the drag surface.
        self.menu_focus.focus(window, cx);
        log::info!(target: "ui-menu", "Opening native menu: {}", menu.name);
        NativeMenu::from(menu).show(position, window, cx);
    }

    fn handle_menu_action(
        &mut self,
        action: &ToolbarAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        log::info!(target: "ui-menu", "{action:?}");
        match *action {
            ToolbarAction::Choose { choice, index } => self.choose(choice, index, window, cx),
            ToolbarAction::Toggle { index, value } => self.toggle(index, value, cx),
            ToolbarAction::Preferences => self.action(4, window, cx),
            ToolbarAction::OutputDirectory => self.open_save_directory(cx),
            ToolbarAction::RefreshScreens => {
                if !self.busy() && self.preferences.is_none() {
                    if let Err(error) = self.refresh_screens(cx) {
                        self.fail(error);
                    }
                    cx.notify();
                }
            }
            ToolbarAction::Minimize => window.minimize_window(),
            ToolbarAction::Quit => self.action(5, window, cx),
        }
    }

    fn native_more_menu(&self) -> Menu {
        let config = &self.state.config;
        let caps = recording_backend().capabilities();
        let locked = self.busy() || self.preferences.is_some();
        let can_toggle = self.preferences.is_none()
            && !matches!(self.phase, Phase::Starting | Phase::Finalizing);
        let mut items = vec![
            MenuItem::action(text("设置", "Preferences"), ToolbarAction::Preferences)
                .disabled(locked),
            MenuItem::action(
                text("打开保存目录", "Open save folder"),
                ToolbarAction::OutputDirectory,
            ),
            MenuItem::action(
                text("刷新显示器", "Refresh displays"),
                ToolbarAction::RefreshScreens,
            )
            .disabled(locked),
            MenuItem::Separator,
        ];
        for (label, choice, selected, values) in [
            (
                text("输出格式", "Output format"),
                Choice::Format,
                config.output_format.index() as usize,
                vec!["MP4".into(), "GIF".into()],
            ),
            (
                text("开始倒计时", "Countdown"),
                Choice::Countdown,
                config.countdown_seconds as usize,
                (0..=10).map(delay_label).collect(),
            ),
        ] {
            let title = format!("{label} · {}", values[selected]);
            items.push(MenuItem::Submenu(choice_menu(
                title, choice, selected, &values, !locked,
            )));
        }
        items.extend([
            MenuItem::Separator,
            toggle_item(
                2,
                text("显示鼠标指针", "Show cursor"),
                config.show_cursor,
                can_toggle,
            ),
            toggle_item(
                3,
                text("高亮鼠标点击", "Highlight clicks"),
                config.highlight_clicks,
                caps.highlight_clicks && can_toggle,
            ),
            MenuItem::Separator,
            MenuItem::action(text("最小化", "Minimize"), ToolbarAction::Minimize),
            MenuItem::action(text("退出", "Quit"), ToolbarAction::Quit),
        ]);
        Menu::new(text("更多操作", "More actions")).items(items)
    }

    fn native_source_menu(&self) -> Menu {
        let mode = self.state.config.source_mode;
        let values = self
            .monitors
            .as_ref()
            .map(|monitors| monitors.labels())
            .unwrap_or_default();
        let selected = self
            .selected_screen
            .and_then(|bounds| self.monitors.as_ref()?.index_of(bounds));
        let mut items = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                MenuItem::action(
                    value.clone(),
                    ToolbarAction::Choose {
                        choice: Choice::Screen,
                        index,
                    },
                )
                .checked(mode == 0 && selected == Some(index))
            })
            .collect::<Vec<_>>();
        if !values.is_empty() {
            items.push(MenuItem::Separator);
        }
        for (index, label) in [
            (1, text("选择窗口…", "Choose window…")),
            (2, text("选择区域…", "Choose region…")),
        ] {
            items.push(
                MenuItem::action(
                    label,
                    ToolbarAction::Choose {
                        choice: Choice::Source,
                        index,
                    },
                )
                .checked(mode as usize == index),
            );
        }
        Menu::new(text("录制范围", "Recording source")).items(items)
    }

    fn source_menu(&self, button: Button, cx: &Context<Self>) -> AnyElement {
        button
            .disabled(self.busy() || self.preferences.is_some())
            .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                view.show_native_menu(view.native_source_menu(), event.position(), window, cx);
            }))
            .into_any_element()
    }

    fn toolbar_summary(&self, cx: &Context<Self>) -> AnyElement {
        let show_status = !self.status.is_empty()
            && (self.error || matches!(self.phase, Phase::Idle | Phase::Recording | Phase::Paused));
        if !show_status
            && self.phase == Phase::Idle
            && let Some(path) = &self.state.last_output
        {
            let path = path.clone();
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();
            return Button::new("last-output")
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .child(format!("{} · {name}", text("已保存", "Saved"))),
                )
                .tooltip(path.to_string_lossy().into_owned())
                .small()
                .ghost()
                .w_full()
                .h(px(18.))
                .min_w_0()
                .max_w_full()
                .px_0()
                .on_click(cx.listener(move |view, _, _, cx| {
                    if let Err(error) = desktop_integration().open_path(&path) {
                        view.fail(error);
                        cx.notify();
                    }
                }))
                .into_any_element();
        }
        let (title, detail, _) = self.target_description();
        let config = &self.state.config;
        let summary = if show_status {
            self.status.clone()
        } else {
            format!(
                "{} · {} · {} · {} FPS",
                if self.phase == Phase::Idle {
                    title.as_str()
                } else {
                    phase_label(self.phase)
                },
                config.output_format.extension().to_uppercase(),
                quality_labels()[config.quality_preset as usize],
                config.output_format.frames_per_second(config.frame_rate),
            )
        };
        let description = format!(
            "{summary}\n{title} · {detail}\n{}: {}\n{}",
            text("倒计时", "Countdown"),
            delay_label(config.countdown_seconds),
            shortcut_hint(config, self.phase),
        );
        self.source_menu(
            Button::new("target-summary")
                .small()
                .ghost()
                .h(px(18.))
                .w_full()
                .min_w_0()
                .px_0()
                .text_color(if self.error {
                    cx.theme().danger
                } else {
                    cx.theme().muted_foreground
                })
                .accessibility_label(format!(
                    "{}: {summary}",
                    text("选择录制目标", "Choose recording target")
                ))
                .tooltip(description)
                .child(div().flex_1().min_w_0().truncate().child(summary)),
            cx,
        )
    }
}

impl Render for RecorderView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let config = &self.state.config;
        let idle = self.phase == Phase::Idle;
        let can_toggle = self.preferences.is_none()
            && !matches!(self.phase, Phase::Starting | Phase::Finalizing);
        let caps = recording_backend().capabilities();
        let counter = if self.phase == Phase::Countdown {
            let seconds = self
                .deadline
                .map(|deadline| {
                    deadline
                        .saturating_duration_since(std::time::Instant::now())
                        .as_secs()
                        + 1
                })
                .unwrap_or(0);
            super::format_duration(std::time::Duration::from_secs(seconds))
        } else {
            super::format_duration(self.elapsed)
        };
        let range = if idle {
            self.source_menu(
                toolbar_button("source-picker")
                    .icon(icon(source_icon(config.source_mode)))
                    .accessibility_label(text("选择录制范围", "Choose recording source"))
                    .tooltip(text(
                        "选择屏幕、窗口或区域",
                        "Choose a display, window, or region",
                    )),
                cx,
            )
        } else {
            let paused = self.phase == Phase::Paused;
            let label = if paused {
                text("继续录制", "Resume recording")
            } else {
                text("暂停录制", "Pause recording")
            };
            let tooltip = match config.pause_hotkey.as_deref() {
                Some(key) => format!("{label} ({})", display_shortcut(Some(key))),
                None => label.to_owned(),
            };
            toolbar_button("pause")
                .icon(icon(if paused {
                    IconName::Play
                } else {
                    IconName::Pause
                }))
                .accessibility_label(label)
                .tooltip(tooltip)
                .disabled(
                    !matches!(self.phase, Phase::Recording | Phase::Paused)
                        || self.preferences.is_some(),
                )
                .on_click(cx.listener(|view, _, window, cx| view.action(2, window, cx)))
                .into_any_element()
        };
        let mut controls = h_flex()
            .gap_2()
            .flex_shrink_0()
            .child(toolbar_cell(
                range,
                if idle {
                    text("范围", "Source")
                } else if self.phase == Phase::Paused {
                    text("继续", "Resume")
                } else {
                    text("暂停", "Pause")
                },
                cx,
            ))
            .child(toolbar_cell(
                self.option_button(
                    "quality",
                    text("画质", "Quality"),
                    Choice::Quality,
                    config.quality_preset as usize,
                    quality_labels()
                        .iter()
                        .map(|label| (*label).into())
                        .collect(),
                    cx,
                ),
                text("画质", "Quality"),
                cx,
            ))
            .child(toolbar_cell(
                self.option_button(
                    "frame-rate",
                    text("帧率", "FPS"),
                    Choice::FrameRate,
                    config.frame_rate as usize,
                    (0..2)
                        .map(|index| {
                            format!("{} FPS", config.output_format.frames_per_second(index))
                        })
                        .collect(),
                    cx,
                ),
                text("帧率", "FPS"),
                cx,
            ));
        for (index, id, name, label, symbol, checked, enabled) in [
            (
                0,
                "system-audio",
                text("系统声音", "System audio"),
                text("系统", "Audio"),
                if config.system_audio {
                    IconName::Volume2
                } else {
                    IconName::VolumeOff
                },
                config.system_audio,
                caps.system_audio && config.output_format.supports_audio(),
            ),
            (
                1,
                "microphone",
                text("麦克风", "Microphone"),
                text("麦克风", "Mic"),
                if config.microphone {
                    IconName::Mic
                } else {
                    IconName::MicOff
                },
                config.microphone,
                caps.microphone && config.output_format.supports_audio(),
            ),
        ] {
            controls =
                controls.child(toolbar_cell(
                    Toggle::new(id)
                        .small()
                        .outline()
                        .w(px(34.))
                        .h(px(34.))
                        .min_w(px(34.))
                        .rounded_full()
                        .icon(icon(symbol))
                        .checked(checked)
                        .tooltip(format!(
                            "{name}: {}",
                            if checked {
                                text("开", "On")
                            } else {
                                text("关", "Off")
                            }
                        ))
                        .disabled(!can_toggle || !enabled)
                        .on_click(cx.listener(move |view, value: &bool, _, cx| {
                            view.toggle(index, *value, cx)
                        })),
                    label,
                    cx,
                ));
        }
        let primary_label = match self.phase {
            Phase::Idle => text("开始", "Start"),
            Phase::Countdown => text("取消", "Cancel"),
            Phase::Finalizing => text("保存", "Saving"),
            _ => text("停止", "Stop"),
        };
        let primary_description = match self.phase {
            Phase::Idle if self.state.last_output.is_some() => text("再次录制", "Record again"),
            Phase::Idle => text("开始录制", "Start recording"),
            Phase::Countdown => text("取消倒计时", "Cancel countdown"),
            Phase::Finalizing => text("正在保存…", "Saving…"),
            _ => text("停止录制", "Stop recording"),
        };
        let primary_key = if idle {
            config.start_hotkey.as_deref()
        } else {
            config.stop_hotkey.as_deref()
        };
        let primary_tooltip = match primary_key {
            Some(key) if self.phase != Phase::Finalizing => {
                format!("{primary_description} ({})", display_shortcut(Some(key)))
            }
            _ => primary_description.to_owned(),
        };
        controls = controls.child(toolbar_cell(
            toolbar_button("record")
                .icon(icon(if idle {
                    IconName::Play
                } else if self.phase == Phase::Countdown {
                    IconName::X
                } else {
                    IconName::Square
                }))
                .danger()
                .bg(cx.theme().danger)
                .text_color(cx.theme().danger_foreground)
                .accessibility_label(primary_description)
                .tooltip(primary_tooltip)
                .disabled(
                    self.phase == Phase::Finalizing
                        || self.preferences.is_some()
                        || (idle && (self.selector.is_some() || self.selected_target().is_none())),
                )
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.action(if idle { 1 } else { 3 }, window, cx)
                })),
            primary_label,
            cx,
        ));
        div()
            .id("recorder-toolbar")
            .track_focus(&self.menu_focus)
            .on_action(cx.listener(Self::handle_menu_action))
            .on_mouse_down(MouseButton::Right, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|view, event: &MouseUpEvent, window, cx| {
                    // As in GPUI's native input menus, show after releasing the
                    // right button so its release does not dismiss the OS menu.
                    cx.stop_propagation();
                    view.show_native_menu(view.native_more_menu(), event.position, window, cx);
                }),
            )
            .size_full()
            .child(
                h_flex()
                    .size_full()
                    .px_4()
                    .gap_3()
                    .bg(cx.theme().background)
                    .text_color(cx.theme().foreground)
                    .child(
                        drag_surface()
                            .relative()
                            .size(px(40.))
                            .flex_shrink_0()
                            .child(Icon::new(IconName::Scan).size(px(40.)))
                            .child(
                                div()
                                    .absolute()
                                    .left(px(15.))
                                    .top(px(15.))
                                    .size(px(10.))
                                    .rounded_full()
                                    .bg(if self.phase == Phase::Paused {
                                        cx.theme().muted_foreground
                                    } else {
                                        cx.theme().danger
                                    }),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                drag_surface()
                                    .text_size(px(31.))
                                    .line_height(px(36.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .font_features(FontFeatures(std::sync::Arc::new(vec![(
                                        "tnum".into(),
                                        1,
                                    )])))
                                    .child(counter),
                            )
                            .child(self.toolbar_summary(cx)),
                    )
                    .child(
                        div()
                            .w(px(1.))
                            .h(px(46.))
                            .flex_shrink_0()
                            .bg(cx.theme().border),
                    )
                    .child(controls),
            )
    }
}

fn source_icon(mode: u8) -> IconName {
    match mode {
        0 => IconName::Monitor,
        1 => IconName::AppWindow,
        _ => IconName::Scan,
    }
}

fn quality_labels() -> [&'static str; 4] {
    [
        text("自动", "Auto"),
        "720p",
        "1080p",
        text("原始", "Original"),
    ]
}

fn delay_label(seconds: u8) -> String {
    if seconds == 0 {
        text("关闭", "Off").into()
    } else {
        format!("{seconds}s")
    }
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => text("准备录制", "Ready"),
        Phase::Countdown => text("即将开始", "Starting soon"),
        Phase::Starting => text("正在启动", "Starting"),
        Phase::Recording => text("录制中", "Recording"),
        Phase::Paused => text("已暂停", "Paused"),
        Phase::Finalizing => text("正在保存", "Saving"),
    }
}

fn shortcut_hint(config: &Config, phase: Phase) -> String {
    let controls = match phase {
        Phase::Idle => vec![(config.start_hotkey.as_deref(), text("开始录制", "Record"))],
        Phase::Recording | Phase::Paused => vec![
            (
                config.pause_hotkey.as_deref(),
                text("暂停 / 继续", "Pause / Resume"),
            ),
            (config.stop_hotkey.as_deref(), text("停止", "Stop")),
        ],
        Phase::Countdown => vec![(config.stop_hotkey.as_deref(), text("取消", "Cancel"))],
        Phase::Starting => vec![(config.stop_hotkey.as_deref(), text("停止", "Stop"))],
        Phase::Finalizing => Vec::new(),
    };
    controls
        .into_iter()
        .filter_map(|(key, label)| {
            key.map(|key| format!("{} {label}", display_shortcut(Some(key))))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::target_for_source;
    use crate::domain::{Bounds as CaptureBounds, RecordingTarget, WindowId};

    #[test]
    fn source_switch_never_reuses_a_target_of_the_wrong_kind() {
        let screen = CaptureBounds {
            left: 0,
            top: 0,
            width: 1920,
            height: 1080,
        };
        let region = CaptureBounds {
            left: -1416,
            top: 275,
            width: 425,
            height: 280,
        };
        let window = RecordingTarget::Window {
            id: WindowId::from_platform_value(7),
            initial_bounds: region,
        };
        assert_eq!(
            target_for_source(0, Some(screen), Some(window)),
            Some(RecordingTarget::Screen(screen))
        );
        assert_eq!(
            target_for_source(1, Some(screen), Some(window)),
            Some(window)
        );
        assert_eq!(target_for_source(2, Some(screen), Some(window)), None);
        assert_eq!(
            target_for_source(1, Some(screen), Some(RecordingTarget::Region(region))),
            None
        );
        assert_eq!(
            target_for_source(2, Some(screen), Some(RecordingTarget::Region(region))),
            Some(RecordingTarget::Region(region))
        );
        assert_eq!(target_for_source(0, None, Some(window)), None);
    }
}
