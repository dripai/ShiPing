use super::{
    hotkeys::{RecordingHotkeys, skip_conflicting_shortcuts},
    overlay::{self, SelectionEvent},
    preferences::Preferences,
    theme,
    tray::RecordingTray,
};
use crate::{
    application::{ApplicationState, Command, Event, RecorderHandle, RecordingOptions},
    config::{Config, OutputFormat},
    domain::{AudioSourceKind, Bounds as CaptureBounds, MonitorCandidates, RecordingTarget},
    i18n::{self, text},
    platform::{desktop_integration, native_window_id, recording_backend, target_selection},
    ports::RecordingCapabilities,
};
use anyhow::{Result, anyhow};
use gpui::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

#[path = "main_panel.rs"]
mod main_panel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Idle,
    Countdown,
    Starting,
    Recording,
    Paused,
    Finalizing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Choice {
    Source,
    Screen,
    Quality,
    FrameRate,
    Format,
    Countdown,
}

pub(super) struct RecorderView {
    pub(super) state: ApplicationState,
    pub(super) phase: Phase,
    menu_focus: FocusHandle,
    hotkeys: Option<RecordingHotkeys>,
    tray: RecordingTray,
    monitors: Option<MonitorCandidates>,
    selected_screen: Option<CaptureBounds>,
    selected_window_title: Option<String>,
    selector: Option<Vec<AnyWindowHandle>>,
    selection_events: Option<Receiver<SelectionEvent>>,
    indicator: Option<AnyWindowHandle>,
    indicator_capture: Option<CaptureBounds>,
    preferences: Option<AnyWindowHandle>,
    deadline: Option<Instant>,
    elapsed: Duration,
    status: String,
    error: bool,
    quitting: bool,
}

pub(crate) fn run() -> Result<()> {
    let startup = Rc::new(RefCell::new(Ok(())));
    let result = startup.clone();
    gpui::application()
        .with_assets(gpui::assets::AllAssets)
        .run(move |cx| {
            gpui::init(cx);
            let mut initial_error = None;
            let mut config = match Config::load() {
                Ok(config) => config,
                Err(error) => {
                    initial_error = Some(error.to_string());
                    Config::default()
                }
            };
            i18n::prepare(config.language);
            log::info!(
                "Settings loaded from {}; source_mode={}; theme={:?}",
                Config::path().display(),
                config.source_mode,
                config.theme
            );
            theme::apply(config.theme, cx);
            normalize_recording_capabilities(&mut config, recording_backend().capabilities());
            let tray = match RecordingTray::new() {
                Ok(tray) => tray,
                Err(error) => {
                    *startup.borrow_mut() = Err(error);
                    cx.quit();
                    return;
                }
            };
            let hotkeys = match RecordingHotkeys::new() {
                Ok(mut hotkeys) => {
                    match skip_conflicting_shortcuts(config.hotkeys(), |values| {
                        hotkeys.reconfigure(values)
                    }) {
                        Ok((registered, conflicts)) => {
                            if registered != config.hotkeys() {
                                config.set_hotkeys(registered);
                                if let Err(error) = config.save() {
                                    initial_error = Some(error.to_string());
                                }
                            }
                            if !conflicts.is_empty() {
                                initial_error = Some(
                                    conflicts
                                        .iter()
                                        .map(|issue| issue.message.as_str())
                                        .collect::<Vec<_>>()
                                        .join("; "),
                                );
                            }
                        }
                        Err(error) => initial_error = Some(error.to_string()),
                    }
                    Some(hotkeys)
                }
                Err(error) => {
                    initial_error = Some(error.to_string());
                    None
                }
            };
            let opened = gpui::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(680.), px(94.)), cx)),
                    // The stock TitleBar includes window controls. The recorder uses a
                    // single toolbar instead; only its timer/logo are native drag areas.
                    titlebar: None,
                    app_owns_titlebar_drag: true,
                    is_resizable: false,
                    window_background: WindowBackgroundAppearance::Opaque,
                    window_min_size: Some(size(px(680.), px(94.))),
                    app_id: Some("com.dripai.shiping".into()),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    window.set_window_title("ShiPing");
                    theme::observe_system_appearance(window);
                    window.on_window_should_close(cx, |window, _| {
                        window.minimize_window();
                        false
                    });
                    cx.new(|cx: &mut Context<RecorderView>| {
                        let mut view = RecorderView {
                            state: ApplicationState::new(config),
                            phase: Phase::Idle,
                            menu_focus: cx.focus_handle(),
                            hotkeys,
                            tray,
                            monitors: None,
                            selected_screen: None,
                            selected_window_title: None,
                            selector: None,
                            selection_events: None,
                            indicator: None,
                            indicator_capture: None,
                            preferences: None,
                            deadline: None,
                            elapsed: Duration::ZERO,
                            status: String::new(),
                            error: false,
                            quitting: false,
                        };
                        if let Err(error) = view.refresh_screens(cx) {
                            view.fail(error);
                        }
                        if let Some(error) = initial_error {
                            view.fail(error);
                        }
                        cx.spawn_in(window, async move |this, cx| {
                            loop {
                                cx.background_executor()
                                    .timer(Duration::from_millis(50))
                                    .await;
                                if this
                                    .update_in(cx, |this, window, cx| this.tick(window, cx))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        })
                        .detach();
                        view
                    })
                },
            );
            if let Err(error) = opened {
                *startup.borrow_mut() = Err(error);
                cx.quit();
                return;
            }
            cx.activate(true);
        });
    Rc::try_unwrap(result).unwrap().into_inner()
}

impl RecorderView {
    pub(super) fn busy(&self) -> bool {
        self.phase != Phase::Idle || self.selector.is_some()
    }
    fn fail(&mut self, error: impl std::fmt::Display) {
        self.status = error.to_string();
        log::error!(target: "ui", "{}", self.status);
        self.error = true;
    }
    fn notice(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.error = false;
    }

    fn refresh_screens(&mut self, cx: &App) -> Result<()> {
        let monitors = target_selection().monitors(Some(cx))?;
        let primary = target_selection().primary_screen_bounds()?;
        let index = self
            .selected_screen
            .and_then(|bounds| monitors.index_of(bounds))
            .unwrap_or_else(|| {
                monitors
                    .index_of(primary)
                    .unwrap_or_else(|| monitors.primary_index())
            });
        self.selected_screen = Some(
            monitors
                .get(index)
                .ok_or_else(|| anyhow!(text("未找到显示器", "No display was found")))?
                .bounds,
        );
        self.monitors = Some(monitors);
        Ok(())
    }

    fn close_selector(&mut self, cx: &mut App) {
        if let Some(handles) = self.selector.take() {
            log::info!(target: "selection", "Closing {} selector window(s)", handles.len());
            for handle in handles {
                if let Err(error) = handle.update(cx, |_, window, _| window.remove_window()) {
                    log::warn!(target: "selection", "Failed to close selector: {error:#}");
                }
            }
            log::info!(target: "selection", "Selector windows closed");
        }
        self.selection_events = None;
    }

    fn clear_indicator(&mut self, cx: &mut App) {
        if let Some(handle) = self.indicator.take() {
            log::info!(target: "indicator", "Closing indicator");
            if let Err(error) = handle.update(cx, |_, window, _| window.remove_window()) {
                log::warn!(target: "indicator", "Failed to close indicator: {error:#}");
            }
        }
        self.indicator_capture = None;
    }

    fn refresh_indicator(&mut self, cx: &mut Context<Self>) -> Result<()> {
        let Some(target) = self.state.target else {
            return Ok(());
        };
        if matches!(target, RecordingTarget::Screen(_)) || self.selector.is_some() {
            return Ok(());
        }
        let capture = target_selection().current_bounds(target)?;
        if self.indicator_capture == Some(capture) {
            return Ok(());
        }
        log::info!(target: "indicator", "Updating indicator: {capture:?}");
        let label = if matches!(target, RecordingTarget::Window { .. }) {
            text("录制窗口", "Recording window")
        } else {
            text("录制区域", "Recording region")
        };
        // Create successfully before releasing the previous indicator.
        let owner = cx.entity().downgrade();
        let new = overlay::open_indicator(
            capture,
            target,
            label,
            move |error, cx| {
                if let Err(error) = owner.update(cx, |this, cx| {
                    this.clear_indicator(cx);
                    this.fail(error);
                    cx.notify();
                }) {
                    log::warn!(target: "indicator", "Could not report placement failure: {error:#}");
                }
            },
            cx,
        )?;
        self.clear_indicator(cx);
        self.indicator = Some(new);
        self.indicator_capture = Some(capture);
        Ok(())
    }

    fn choose_source(
        &mut self,
        mode: u8,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        log::info!(target: "selection", "Choosing source mode={mode}");
        if self.busy() {
            return Err(anyhow!(text(
                "当前不能更改录制目标",
                "Cannot change the recording target now"
            )));
        }
        if mode == 0 {
            self.refresh_screens(cx)?;
            self.clear_indicator(cx);
            self.state.config.source_mode = 0;
            self.state.target = self.selected_screen.map(RecordingTarget::Screen);
            self.selected_window_title = None;
            return Ok(());
        }
        #[cfg(target_os = "macos")]
        let surfaces = crate::platform::selection_surfaces()?;
        #[cfg(not(target_os = "macos"))]
        let surfaces = vec![target_selection().virtual_desktop_bounds()?];
        let mut selections = Vec::new();
        for desktop in surfaces {
            let candidates = if mode == 1 {
                let mut candidates = target_selection().windows(desktop)?;
                if let Some(id) = native_window_id(window) {
                    candidates.exclude(id);
                }
                Some(candidates)
            } else {
                None
            };
            selections.push((desktop, candidates));
        }
        let (sender, receiver) = mpsc::channel();
        self.selector = Some(Vec::new());
        for (desktop, candidates) in selections {
            match overlay::open_selector(desktop, candidates, sender.clone(), cx) {
                Ok(handle) => self
                    .selector
                    .as_mut()
                    .expect("selector collection initialized")
                    .push(handle),
                Err(error) => {
                    self.close_selector(cx);
                    return Err(error);
                }
            }
        }
        self.clear_indicator(cx);
        self.selection_events = Some(receiver);
        window.minimize_window();
        Ok(())
    }

    fn options(&self) -> Result<RecordingOptions> {
        let config = &self.state.config;
        let target = self.selected_target().ok_or_else(|| {
            anyhow!(if config.source_mode == 0 {
                text("请先选择显示器", "Select a display first")
            } else {
                text("请先选择录制目标", "Select a recording target first")
            })
        })?;
        target.initial_bounds().validate()?;
        target_selection().current_bounds(target)?.validate()?;
        let caps = recording_backend().capabilities();
        Ok(RecordingOptions {
            target,
            quality_preset: config.quality_preset,
            frames_per_second: config.output_format.frames_per_second(config.frame_rate),
            output_format: config.output_format,
            system_audio: config.output_format.supports_audio()
                && caps.system_audio
                && config.system_audio,
            microphone: config.output_format.supports_audio()
                && caps.microphone
                && config.microphone,
            show_cursor: config.show_cursor,
            highlight_clicks: caps.highlight_clicks && config.highlight_clicks,
            save_directory: config.save_directory.clone(),
        })
    }

    fn start(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.preferences.is_some() {
            return Err(anyhow!(text(
                "请先保存或关闭设置",
                "Save or close preferences first"
            )));
        }
        if self.busy() {
            return Err(anyhow!(text(
                "已有录制任务或目标选择正在进行",
                "Recording or target selection is already active"
            )));
        }
        let options = self.options()?;
        log::info!(target: "recording", "Starting recording: target={:?}; format={:?}; fps={}; system_audio={}; microphone={}", options.target, options.output_format, options.frames_per_second, options.system_audio, options.microphone);
        self.state.config.save()?;
        self.state.target = Some(options.target);
        self.refresh_indicator(cx)?;
        self.state.pending_options = Some(options);
        self.state.last_output = None;
        self.elapsed = Duration::ZERO;
        self.phase = Phase::Countdown;
        self.deadline =
            Some(Instant::now() + Duration::from_secs(self.state.config.countdown_seconds.into()));
        Ok(())
    }

    fn stop(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        log::info!(target: "recording", "Stop requested; phase={:?}", self.phase);
        if self.phase == Phase::Countdown {
            self.deadline = None;
            self.state.pending_options = None;
            self.phase = Phase::Idle;
            self.notice(text("已取消开始录制", "Recording start canceled"));
        } else if let Some(recorder) = &self.state.recorder {
            recorder.send(Command::Stop);
            self.phase = Phase::Finalizing;
            self.notice(text("正在完成录制文件…", "Finalizing recording…"));
        }
        self.clear_indicator(cx);
        window.activate_window();
    }

    fn quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = self.state.config.save() {
            self.fail(error);
            return;
        }
        self.close_selector(cx);
        self.quitting = true;
        self.stop(window, cx);
        if self.state.recorder.is_none() {
            cx.quit();
        }
    }

    pub(super) fn save_preferences(
        &mut self,
        mut new: Config,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        if self.busy() {
            return Err(anyhow!(text(
                "录制或选择期间不能保存设置",
                "Cannot save preferences while recording or selecting"
            )));
        }
        if new.save_directory.as_os_str().is_empty() {
            return Err(anyhow!(text(
                "保存目录不能为空",
                "Save folder cannot be empty"
            )));
        }
        normalize_recording_capabilities(&mut new, recording_backend().capabilities());
        let old = self.state.config.clone();
        match self.hotkeys.as_mut() {
            Some(hotkeys) => {
                let registered = hotkeys.reconfigure(new.hotkeys())?;
                new.set_hotkeys(registered);
            }
            None if new.hotkeys().iter().any(Option::is_some) => {
                return Err(anyhow!(text(
                    "全局快捷键管理器不可用",
                    "Global shortcut manager is unavailable"
                )));
            }
            None => {}
        }
        if let Err(error) = new.save() {
            if let Some(hotkeys) = self.hotkeys.as_mut() {
                let previous = if self.preferences.is_some() {
                    [None, None, None]
                } else {
                    old.hotkeys()
                };
                hotkeys
                    .reconfigure(previous)
                    .map_err(|rollback| anyhow!("{error}; shortcut rollback failed: {rollback}"))?;
            }
            return Err(error);
        }
        i18n::prepare(new.language);
        theme::apply(new.theme, cx);
        self.state.config = new;
        self.preferences = None;
        self.clear_indicator(cx);
        self.notice(text("设置已保存", "Preferences saved"));
        self.tray.refresh(false, false);
        cx.notify();
        Ok(())
    }

    fn open_preferences(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if self.busy() {
            return Err(anyhow!(text(
                "录制或选择期间不能打开设置",
                "Cannot open preferences while recording or selecting"
            )));
        }
        if let Some(handle) = self.preferences
            && handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        {
            return Ok(());
        }
        if let Some(hotkeys) = self.hotkeys.as_mut() {
            hotkeys.reconfigure([None, None, None])?;
        }
        let owner = cx.entity().downgrade();
        let config = self.state.config.clone();
        let opened = gpui::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(560.), px(420.)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some(text("拾屏设置", "ShiPing Preferences").into()),
                    ..Default::default()
                }),
                window_min_size: Some(size(px(520.), px(360.))),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Preferences::new(config, owner, window, cx)),
        );
        let (handle, _) = match opened {
            Ok(value) => value,
            Err(error) => {
                self.resume_shortcuts(cx)?;
                return Err(error);
            }
        };
        self.preferences = Some(handle);
        Ok(())
    }

    pub(super) fn resume_shortcuts(&mut self, cx: &mut Context<Self>) -> Result<()> {
        if let Some(hotkeys) = self.hotkeys.as_mut()
            && let Err(error) = hotkeys.reconfigure(self.state.config.hotkeys())
        {
            self.fail(&error);
            cx.notify();
            return Err(error.into());
        }
        self.preferences = None;
        cx.notify();
        Ok(())
    }

    fn action(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        log::info!(target: "ui", "Action={index}; phase={:?}", self.phase);
        let result = match index {
            0 => {
                window.activate_window();
                Ok(())
            }
            1 => self.start(cx),
            2 => {
                if matches!(self.phase, Phase::Recording | Phase::Paused)
                    && let Some(recorder) = &self.state.recorder
                {
                    recorder.send(Command::TogglePause);
                }
                Ok(())
            }
            3 => {
                self.stop(window, cx);
                Ok(())
            }
            4 => self.open_preferences(cx),
            5 => {
                self.quit(window, cx);
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(error);
        }
        cx.notify();
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selection = self
            .selection_events
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(event) = selection {
            log::info!(target: "selection", "Processing event: {event:?}");
            self.close_selector(cx);
            match event {
                SelectionEvent::Selected {
                    target,
                    window_title,
                } => {
                    self.state.config.source_mode =
                        if matches!(target, RecordingTarget::Window { .. }) {
                            1
                        } else {
                            2
                        };
                    self.state.target = Some(target);
                    self.selected_window_title = window_title;
                    self.notice(String::new());
                }
                SelectionEvent::Canceled => {
                    self.notice(text("已取消目标选择", "Target selection canceled"))
                }
                SelectionEvent::Failed(error) => self.fail(error),
            }
            if let Err(error) = self.refresh_indicator(cx) {
                self.fail(error);
            }
            log::info!(target: "selection", "Restoring main window after selection");
            window.activate_window();
            cx.notify();
        }
        if let Some(deadline) = self.deadline {
            if Instant::now() >= deadline {
                self.deadline = None;
                if let Some(options) = self.state.pending_options.take() {
                    match RecorderHandle::start(options) {
                        Ok(recorder) => {
                            self.state.recorder = Some(recorder);
                            self.phase = Phase::Starting;
                            self.notice(text(
                                "正在初始化录制设备…",
                                "Initializing recording devices…",
                            ));
                        }
                        Err(error) => {
                            self.phase = Phase::Idle;
                            self.clear_indicator(cx);
                            self.fail(error);
                        }
                    }
                }
                cx.notify();
            } else {
                let seconds = deadline.saturating_duration_since(Instant::now()).as_secs() + 1;
                let status = format!(
                    "{seconds} {}",
                    text("秒后开始录制", "seconds until recording starts")
                );
                if status != self.status {
                    self.notice(status);
                    cx.notify();
                }
            }
        }
        let events = self
            .state
            .recorder
            .as_ref()
            .map(RecorderHandle::drain_events)
            .unwrap_or_default();
        for event in events {
            if !matches!(&event, Event::Progress(_)) {
                log::info!(target: "recording", "{event:?}");
            }
            match event {
                Event::Started {
                    output_path,
                    system_available,
                    microphone_available,
                    warnings,
                } => {
                    let _ = (output_path, system_available, microphone_available);
                    // Stop may already have been queued while the backend was initializing.
                    if self.phase != Phase::Finalizing {
                        self.phase = Phase::Recording;
                        if self.state.config.auto_minimize_after_start {
                            window.minimize_window();
                        }
                        self.notice(if warnings.is_empty() {
                            String::new()
                        } else {
                            warnings.join("; ")
                        });
                    }
                }
                Event::Progress(duration) => self.elapsed = duration,
                Event::Paused(paused) => {
                    if self.phase != Phase::Finalizing {
                        self.phase = if paused {
                            Phase::Paused
                        } else {
                            Phase::Recording
                        };
                        self.notice(String::new());
                    }
                }
                Event::AudioRejected(source, reason) => {
                    match source {
                        AudioSourceKind::System => self.state.config.system_audio = false,
                        AudioSourceKind::Microphone => self.state.config.microphone = false,
                    };
                    self.fail(reason);
                }
                Event::Completed {
                    output_path,
                    duration,
                } => {
                    self.clear_indicator(cx);
                    self.state.recorder.take();
                    self.phase = Phase::Idle;
                    self.elapsed = duration;
                    self.notice(String::new());
                    if self.state.config.open_directory_after_stop
                        && let Some(parent) = output_path.parent()
                        && let Err(error) = desktop_integration().open_path(parent)
                    {
                        self.fail(error);
                    }
                    self.state.last_output = Some(output_path);
                    if self.quitting {
                        cx.quit();
                    }
                }
                Event::Failed(error) => {
                    self.clear_indicator(cx);
                    self.state.recorder.take();
                    self.phase = Phase::Idle;
                    self.quitting = false;
                    window.activate_window();
                    self.fail(error);
                }
            }
            cx.notify();
        }
        if self.indicator.is_some()
            && let Err(error) = self.refresh_indicator(cx)
        {
            self.clear_indicator(cx);
            self.fail(error);
            cx.notify();
        }
        let actions = self
            .hotkeys
            .as_ref()
            .map(RecordingHotkeys::drain_actions)
            .unwrap_or_default();
        for action in actions {
            self.action(action + 1, window, cx);
        }
        for action in self.tray.drain_actions() {
            self.action(action, window, cx);
        }
        self.tray.refresh(
            self.busy(),
            matches!(self.phase, Phase::Recording | Phase::Paused),
        );
    }

    fn choose(
        &mut self,
        choice: Choice,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy() || self.preferences.is_some() {
            return;
        }
        let result = match choice {
            Choice::Source => self.choose_source(index as u8, window, cx),
            Choice::Screen => match self.monitors.as_ref().and_then(|values| values.get(index)) {
                Some(monitor) => {
                    self.clear_indicator(cx);
                    self.selected_screen = Some(monitor.bounds);
                    self.state.target = Some(RecordingTarget::Screen(monitor.bounds));
                    self.state.config.source_mode = 0;
                    self.selected_window_title = None;
                    Ok(())
                }
                None => Err(anyhow!(text(
                    "显示器已不可用，请刷新",
                    "Display unavailable; refresh displays"
                ))),
            },
            Choice::Quality => {
                self.state.config.quality_preset = index as u8;
                Ok(())
            }
            Choice::FrameRate => {
                self.state.config.frame_rate = index as u8;
                Ok(())
            }
            Choice::Format => {
                self.state.config.output_format = OutputFormat::from_index(index as i32);
                if index == 1 {
                    self.state.config.system_audio = false;
                    self.state.config.microphone = false;
                }
                Ok(())
            }
            Choice::Countdown => {
                self.state.config.countdown_seconds = index as u8;
                Ok(())
            }
        };
        if let Err(error) = result {
            self.fail(error);
        }
        cx.notify();
    }

    fn toggle(&mut self, index: usize, value: bool, cx: &mut Context<Self>) {
        if self.preferences.is_some() || matches!(self.phase, Phase::Starting | Phase::Finalizing) {
            return;
        }
        let caps = recording_backend().capabilities();
        let command = match index {
            0 => {
                let value =
                    value && caps.system_audio && self.state.config.output_format.supports_audio();
                self.state.config.system_audio = value;
                Command::SystemAudio(value)
            }
            1 => {
                let value =
                    value && caps.microphone && self.state.config.output_format.supports_audio();
                self.state.config.microphone = value;
                Command::Microphone(value)
            }
            2 => {
                self.state.config.show_cursor = value;
                Command::ShowCursor(value)
            }
            _ => {
                let value = value && caps.highlight_clicks;
                self.state.config.highlight_clicks = value;
                Command::HighlightClicks(value)
            }
        };
        if let Some(recorder) = &self.state.recorder {
            recorder.send(command);
        }
        if let Some(options) = &mut self.state.pending_options {
            options.system_audio = self.state.config.system_audio;
            options.microphone = self.state.config.microphone;
            options.show_cursor = self.state.config.show_cursor;
            options.highlight_clicks = self.state.config.highlight_clicks;
        }
        cx.notify();
    }
}

pub(super) fn normalize_recording_capabilities(
    config: &mut Config,
    capabilities: RecordingCapabilities,
) {
    config.system_audio &= capabilities.system_audio && config.output_format.supports_audio();
    config.microphone &= capabilities.microphone && config.output_format.supports_audio();
    config.highlight_clicks &= capabilities.highlight_clicks;
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::normalize_recording_capabilities;
    use crate::{
        config::{Config, OutputFormat},
        ports::RecordingCapabilities,
    };
    #[test]
    fn unavailable_features_and_gif_audio_are_disabled() {
        let mut config = Config {
            system_audio: true,
            microphone: true,
            highlight_clicks: true,
            ..Config::default()
        };
        normalize_recording_capabilities(
            &mut config,
            RecordingCapabilities {
                system_audio: false,
                microphone: true,
                highlight_clicks: false,
            },
        );
        assert!(!config.system_audio);
        assert!(config.microphone);
        assert!(!config.highlight_clicks);
        config.output_format = OutputFormat::Gif;
        normalize_recording_capabilities(
            &mut config,
            RecordingCapabilities {
                system_audio: true,
                microphone: true,
                highlight_clicks: true,
            },
        );
        assert!(!config.system_audio && !config.microphone);
    }
}
