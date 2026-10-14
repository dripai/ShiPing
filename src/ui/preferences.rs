use super::{
    controller::RecorderView,
    hotkeys::{display_shortcut, shortcut_from_key_event},
};
use crate::{
    config::{Config, LanguageMode, OutputFormat, ThemePreference},
    i18n::text,
    platform::{desktop_integration, recording_backend},
};
use gpui::component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    form::{Field, Form},
    h_flex,
    input::{Input, InputState},
    menu::{DropdownMenu, PopupMenuItem},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    switch::Switch,
    v_flex,
};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::path::PathBuf;

pub(super) struct Preferences {
    config: Config,
    owner: WeakEntity<RecorderView>,
    directory: Entity<InputState>,
    active_section: usize,
    capture: Option<usize>,
    focus: FocusHandle,
    status: String,
}

impl Preferences {
    pub(super) fn new(
        config: Config,
        owner: WeakEntity<RecorderView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let directory = cx.new(|cx| {
            let mut input = InputState::new(window, cx);
            input.set_value(
                config.save_directory.to_string_lossy().into_owned(),
                window,
                cx,
            );
            input
        });
        let close_owner = owner.clone();
        window.on_window_should_close(cx, move |_, cx| {
            close_owner
                .update(cx, |owner, cx| owner.resume_shortcuts(cx))
                .is_ok_and(|result| result.is_ok())
        });
        Self {
            config,
            owner,
            directory,
            active_section: 0,
            capture: None,
            focus: cx.focus_handle(),
            status: String::new(),
        }
    }

    fn select(
        &self,
        id: &'static str,
        label: &'static str,
        values: Vec<String>,
        selected: usize,
        set: fn(&mut Config, usize),
        cx: &Context<Self>,
    ) -> Field {
        let owner = cx.entity().downgrade();
        Field::new().label(label).child(
            Button::new(id)
                .label(values[selected].clone())
                .accessibility_label(format!("{label}: {}", values[selected]))
                .small()
                .w_full()
                .max_w(px(180.))
                .min_w_0()
                .justify_between()
                .child(Icon::new(IconName::ChevronDown).size_3())
                .dropdown_menu(move |mut menu, _, _| {
                    for (index, value) in values.iter().enumerate() {
                        let owner = owner.clone();
                        menu = menu.item(
                            PopupMenuItem::new(value.clone())
                                .checked(index == selected)
                                .on_click(move |_, _, cx| {
                                    let _ = owner.update(cx, |this, cx| {
                                        set(&mut this.config, index);
                                        cx.notify();
                                    });
                                }),
                        );
                    }
                    menu
                }),
        )
    }

    fn toggle(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        enabled: bool,
        set: fn(&mut Config, bool),
        cx: &Context<Self>,
    ) -> Field {
        Field::new().label(label).child(
            Switch::new(id)
                .accessibility_label(label)
                .checked(checked)
                .disabled(!enabled)
                .small()
                .on_click(cx.listener(move |this, value: &bool, _, cx| {
                    set(&mut this.config, *value);
                    cx.notify();
                })),
        )
    }

    fn general(&self, cx: &Context<Self>) -> Div {
        let config = &self.config;
        let language = match config.language {
            LanguageMode::System => 0,
            LanguageMode::Chinese => 1,
            LanguageMode::English => 2,
        };
        let theme = match config.theme {
            ThemePreference::System => 0,
            ThemePreference::Light => 1,
            ThemePreference::Dark => 2,
        };
        v_flex()
            .gap_4()
            .child(
                Form::horizontal()
                    .label_width(px(148.))
                    .small()
                    .gap_3()
                    .child(self.select(
                        "theme",
                        text("主题", "Theme"),
                        vec![
                            text("跟随系统", "System").into(),
                            text("浅色", "Light").into(),
                            text("深色", "Dark").into(),
                        ],
                        theme,
                        |c, i| {
                            c.theme = match i {
                                1 => ThemePreference::Light,
                                2 => ThemePreference::Dark,
                                _ => ThemePreference::System,
                            };
                        },
                        cx,
                    ))
                    .child(self.select(
                        "language",
                        text("语言", "Language"),
                        vec![
                            text("跟随系统", "System").into(),
                            "简体中文".into(),
                            "English".into(),
                        ],
                        language,
                        |c, i| {
                            c.language = match i {
                                1 => LanguageMode::Chinese,
                                2 => LanguageMode::English,
                                _ => LanguageMode::System,
                            };
                        },
                        cx,
                    ))
                    .child(self.toggle(
                        "minimize",
                        text("录制开始后最小化", "Minimize on start"),
                        config.auto_minimize_after_start,
                        true,
                        |c, v| c.auto_minimize_after_start = v,
                        cx,
                    ))
                    .child(self.toggle(
                        "open-folder-after-stop",
                        text("录制结束后打开目录", "Open folder after saving"),
                        config.open_directory_after_stop,
                        true,
                        |c, v| c.open_directory_after_stop = v,
                        cx,
                    )),
            )
            .child(
                Form::horizontal().label_width(px(76.)).small().child(
                    Field::new().label(text("保存目录", "Save folder")).child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(Input::new(&self.directory).small()),
                            )
                            .child(
                                Button::new("browse")
                                    .label(text("浏览…", "Browse…"))
                                    .small()
                                    .flex_shrink_0()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let path = this.directory.read(cx).value();
                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_directory(path.as_str())
                                            .pick_folder()
                                        {
                                            this.directory.update(cx, |input, cx| {
                                                input.set_value(
                                                    path.to_string_lossy().into_owned(),
                                                    window,
                                                    cx,
                                                )
                                            });
                                        }
                                    })),
                            )
                            .child(
                                Button::new("open-folder")
                                    .label(text("打开", "Open"))
                                    .tooltip(text("打开保存目录", "Open save folder"))
                                    .small()
                                    .flex_shrink_0()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let value = this.directory.read(cx).value();
                                        if value.trim().is_empty() {
                                            this.status = text(
                                                "保存目录不能为空",
                                                "Save folder cannot be empty",
                                            )
                                            .into();
                                        } else {
                                            let path = PathBuf::from(value.as_str());
                                            let result = std::fs::create_dir_all(&path)
                                                .map_err(anyhow::Error::from)
                                                .and_then(|_| {
                                                    desktop_integration().open_path(&path)
                                                });
                                            this.status = result
                                                .err()
                                                .map(|error| error.to_string())
                                                .unwrap_or_default();
                                        }
                                        cx.notify();
                                    })),
                            ),
                    ),
                ),
            )
    }

    fn recording(&self, cx: &Context<Self>) -> Div {
        let config = &self.config;
        let caps = recording_backend().capabilities();
        v_flex()
            .gap_4()
            .child(
                Form::horizontal()
                    .label_width(px(112.))
                    .small()
                    .gap_3()
                    .child(self.select(
                        "quality",
                        text("画质", "Quality"),
                        vec![
                            text("自动", "Auto").into(),
                            "720p".into(),
                            "1080p".into(),
                            text("原始", "Original").into(),
                        ],
                        config.quality_preset as usize,
                        |c, i| c.quality_preset = i as u8,
                        cx,
                    ))
                    .child(self.select(
                        "format",
                        text("输出格式", "Output format"),
                        vec!["MP4".into(), "GIF".into()],
                        config.output_format.index() as usize,
                        |c, i| {
                            c.output_format = OutputFormat::from_index(i as i32);
                            if !c.output_format.supports_audio() {
                                c.system_audio = false;
                                c.microphone = false;
                            }
                        },
                        cx,
                    ))
                    .child(
                        self.select(
                            "fps",
                            text("帧率", "Frame rate"),
                            (0..2)
                                .map(|i| {
                                    format!("{} FPS", config.output_format.frames_per_second(i))
                                })
                                .collect(),
                            config.frame_rate as usize,
                            |c, i| c.frame_rate = i as u8,
                            cx,
                        ),
                    )
                    .child(self.select(
                        "countdown",
                        text("开始倒计时", "Countdown"),
                        (0..=10).map(|i| format!("{i}s")).collect(),
                        config.countdown_seconds as usize,
                        |c, i| c.countdown_seconds = i as u8,
                        cx,
                    )),
            )
            .child(
                Form::horizontal()
                    .columns(2)
                    .label_width(px(112.))
                    .small()
                    .gap_3()
                    .child(self.toggle(
                        "system-audio",
                        text("系统音频", "System audio"),
                        config.system_audio,
                        caps.system_audio && config.output_format.supports_audio(),
                        |c, v| c.system_audio = v,
                        cx,
                    ))
                    .child(self.toggle(
                        "microphone",
                        text("麦克风", "Microphone"),
                        config.microphone,
                        caps.microphone && config.output_format.supports_audio(),
                        |c, v| c.microphone = v,
                        cx,
                    ))
                    .child(self.toggle(
                        "cursor",
                        text("显示鼠标指针", "Show cursor"),
                        config.show_cursor,
                        true,
                        |c, v| c.show_cursor = v,
                        cx,
                    ))
                    .child(self.toggle(
                        "highlight",
                        text("高亮鼠标点击", "Highlight clicks"),
                        config.highlight_clicks,
                        caps.highlight_clicks,
                        |c, v| c.highlight_clicks = v,
                        cx,
                    )),
            )
    }

    fn shortcuts(&self, cx: &Context<Self>) -> Form {
        let labels = [
            text("开始录制", "Start recording"),
            text("暂停 / 继续", "Pause / Resume"),
            text("停止录制", "Stop recording"),
        ];
        let mut form = Form::horizontal().label_width(px(112.)).small().gap_3();
        for (index, shortcut) in self.config.hotkeys().iter().enumerate() {
            let label = if self.capture == Some(index) {
                text("请按快捷键…", "Press shortcut…").to_owned()
            } else if shortcut.is_none() {
                text("未设置", "Unset").to_owned()
            } else {
                display_shortcut(shortcut.as_deref())
            };
            form = form.child(
                Field::new().label(labels[index]).child(
                    h_flex()
                        .w_full()
                        .min_w_0()
                        .gap_2()
                        .child(
                            Button::new(("shortcut", index))
                                .label(label.clone())
                                .tooltip(label)
                                .small()
                                .flex_1()
                                .min_w_0()
                                .max_w(px(180.))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.capture = Some(index);
                                    this.status.clear();
                                    window.focus(&this.focus, cx);
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new(("clear-shortcut", index))
                                .label(text("清除", "Clear"))
                                .small()
                                .ghost()
                                .flex_shrink_0()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let mut keys = this.config.hotkeys();
                                    keys[index] = None;
                                    this.config.set_hotkeys(keys);
                                    this.capture = None;
                                    cx.notify();
                                })),
                        ),
                ),
            );
        }
        form
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.config.save_directory = PathBuf::from(self.directory.read(cx).value().as_str());
        let config = self.config.clone();
        match self
            .owner
            .update(cx, |owner, cx| owner.save_preferences(config, cx))
        {
            Ok(Ok(())) => window.remove_window(),
            Ok(Err(error)) | Err(error) => {
                self.status = error.to_string();
                cx.notify();
            }
        }
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self
            .owner
            .update(cx, |owner, cx| owner.resume_shortcuts(cx))
        {
            Ok(Ok(())) => window.remove_window(),
            Ok(Err(error)) | Err(error) => {
                self.status = error.to_string();
                cx.notify();
            }
        }
    }
}

impl Render for Preferences {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.active_section {
            1 => self.recording(cx).into_any_element(),
            2 => self.shortcuts(cx).into_any_element(),
            _ => self.general(cx).into_any_element(),
        };
        let navigation = Sidebar::new("preferences-navigation")
            .w(px(96.))
            .collapsible(false)
            .child(
                SidebarMenu::new().gap_1().children(
                    [
                        text("常规", "General"),
                        text("录制", "Recording"),
                        text("快捷键", "Shortcuts"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, label)| {
                        SidebarMenuItem::new(label)
                            .active(index == self.active_section)
                            .px_1()
                            .py_1()
                            .whitespace_nowrap()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.active_section = index;
                                this.capture = None;
                                cx.notify();
                            }))
                    }),
                ),
            );
        v_flex()
            .id("preferences")
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                let Some(index) = this.capture else { return };
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.capture = None;
                    cx.notify();
                    return;
                }
                let mods = event.keystroke.modifiers;
                match shortcut_from_key_event(
                    &event.keystroke.key,
                    mods.control,
                    mods.alt,
                    mods.shift,
                    mods.platform,
                ) {
                    Ok(key) => {
                        let mut keys = this.config.hotkeys();
                        keys[index] = Some(key);
                        this.config.set_hotkeys(keys);
                        this.capture = None;
                        this.status.clear();
                    }
                    Err(error) => this.status = error.to_string(),
                }
                cx.notify();
            }))
            .child(
                h_flex()
                    .flex_1()
                    .w_full()
                    .min_w_0()
                    .min_h_0()
                    .items_stretch()
                    .overflow_hidden()
                    .child(navigation)
                    .child(
                        div()
                            .id(("preferences-content", self.active_section))
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p_4()
                            .child(content),
                    ),
            )
            .child(
                v_flex()
                    .flex_shrink_0()
                    .px_4()
                    .py_2()
                    .gap_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .when(!self.status.is_empty(), |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(self.status.clone()),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                Button::new("defaults")
                                    .label(text("恢复默认", "Restore defaults"))
                                    .small()
                                    .ghost()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.config = Config::default();
                                        this.directory.update(cx, |input, cx| {
                                            input.set_value(
                                                this.config
                                                    .save_directory
                                                    .to_string_lossy()
                                                    .into_owned(),
                                                window,
                                                cx,
                                            )
                                        });
                                        this.capture = None;
                                        this.status.clear();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("cancel")
                                            .label(text("取消", "Cancel"))
                                            .small()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.cancel(window, cx)
                                            })),
                                    )
                                    .child(
                                        Button::new("save")
                                            .label(text("保存", "Save"))
                                            .small()
                                            .primary()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.save(window, cx)
                                            })),
                                    ),
                            ),
                    ),
            )
    }
}
