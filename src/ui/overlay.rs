use crate::{
    domain::{Bounds as CaptureBounds, RecordingTarget, WindowCandidates},
    i18n::text,
    platform::{place_overlay, pointer_position},
};
use anyhow::{Result, anyhow};
use gpui::*;
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub(super) enum SelectionEvent {
    Selected {
        target: RecordingTarget,
        window_title: Option<String>,
    },
    Canceled,
    Failed(String),
}

pub(super) fn options(
    bounds: CaptureBounds,
    target: RecordingTarget,
    interactive: bool,
) -> Result<WindowOptions> {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(bounds.left as f32), px(bounds.top as f32)),
            size(px(bounds.width as f32), px(bounds.height as f32)),
        ))),
        titlebar: None,
        kind: WindowKind::PopUp,
        focus: interactive,
        // Windows places the client area with native calls after creation.
        // Keep indicators hidden until aligned, including during window tracking.
        show: interactive || !cfg!(target_os = "windows"),
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    {
        let (logical, display_id) = crate::platform::overlay_geometry(bounds, target)?;
        return Ok(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(logical)),
            display_id: Some(display_id),
            ..options
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = target;
        Ok(options)
    }
}

pub(super) fn open_selector(
    desktop: CaptureBounds,
    candidates: Option<WindowCandidates>,
    events: Sender<SelectionEvent>,
    cx: &mut App,
) -> Result<AnyWindowHandle> {
    log::info!(target: "selection", "Opening selector: desktop={desktop:?}; window_mode={}", candidates.is_some());
    let close_events = events.clone();
    let failed_events = events.clone();
    let handle = cx.open_window(
        options(desktop, RecordingTarget::Region(desktop), true)?,
        |window, cx| {
            window.on_window_should_close(cx, move |_, _| {
                let _ = close_events.send(SelectionEvent::Canceled);
                true
            });
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                Selector {
                    desktop,
                    candidates,
                    events,
                    start: None,
                    current: None,
                    hover: None,
                    focus,
                    error: String::new(),
                }
            })
        },
    )?;
    log::info!(target: "selection", "Selector created; applying native placement");
    if let Err(error) = handle.update(cx, |_, window, cx| {
        place_overlay(window, desktop, false, cx, move |error, _| {
            let _ = failed_events.send(SelectionEvent::Failed(error.to_string()));
        })
    })? {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        return Err(error);
    }
    log::info!(target: "selection", "Selector placement configured");
    Ok(handle.into())
}

struct Selector {
    desktop: CaptureBounds,
    candidates: Option<WindowCandidates>,
    events: Sender<SelectionEvent>,
    start: Option<(i32, i32)>,
    current: Option<(i32, i32)>,
    hover: Option<crate::domain::WindowCandidate>,
    focus: FocusHandle,
    error: String,
}

impl Selector {
    fn cancel(&mut self) {
        log::info!(target: "selection", "Selection canceled by input");
        let _ = self.events.send(SelectionEvent::Canceled);
    }
    fn position(&mut self, position: Point<Pixels>, window: &Window) -> Option<(i32, i32)> {
        match pointer_position(window, position, self.desktop) {
            Ok(value) => Some(value),
            Err(error) => {
                let _ = self.events.send(SelectionEvent::Failed(error.to_string()));
                None
            }
        }
    }
    fn selected_bounds(&self) -> Option<CaptureBounds> {
        if self.candidates.is_some() {
            self.hover.as_ref().map(|window| window.bounds)
        } else {
            Some(drag_bounds(self.start?, self.current?))
        }
    }
}

impl Render for Selector {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = window.scale_factor();
        let mut root = div()
            .id("target-selector")
            .size_full()
            .relative()
            .bg(rgba(0x10182050))
            .track_focus(&self.focus)
            .cursor_crosshair()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, _| {
                if event.keystroke.key == "escape" {
                    this.cancel();
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, _| this.cancel()),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    if let Some(point) = this.position(event.position, window) {
                        log::info!(target: "selection", "Mouse down: {point:?}");
                        this.start = Some(point);
                        this.current = Some(point);
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if let Some(point) = this.position(event.position, window) {
                    this.current = Some(point);
                    this.hover = this
                        .candidates
                        .as_ref()
                        .and_then(|windows| windows.target_at(point.0, point.1));
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, window, cx| {
                    let Some(end) = this.position(event.position, window) else {
                        return;
                    };
                    this.current = Some(end);
                    log::info!(target: "selection", "Mouse up: start={:?}; end={end:?}", this.start);
                    let target = if let Some(candidates) = &this.candidates {
                        candidates
                            .target_at(end.0, end.1)
                            .map(|candidate| {
                                (
                                    RecordingTarget::Window {
                                        id: candidate.id,
                                        initial_bounds: candidate.bounds,
                                    },
                                    Some(candidate.title),
                                )
                            })
                            .ok_or_else(|| {
                                anyhow!(text("这里没有可录制的窗口", "No recordable window here"))
                            })
                    } else {
                        this.start
                            .ok_or_else(|| anyhow!("No selection start"))
                            .and_then(|start| drag_bounds(start, end).validate())
                            .map(|bounds| (RecordingTarget::Region(bounds), None))
                    };
                    match target {
                        Ok((target, window_title)) => {
                            log::info!(target: "selection", "Sending selected target: {target:?}");
                            let _ = this.events.send(SelectionEvent::Selected {
                                target,
                                window_title,
                            });
                        }
                        Err(error) => {
                            log::warn!(target: "selection", "Invalid selection: {error:#}");
                            this.error = error.to_string();
                        }
                    }
                    cx.notify();
                }),
            );
        if let Some(bounds) = self.selected_bounds() {
            root = root.child(
                div()
                    .absolute()
                    .left(px((bounds.left - self.desktop.left) as f32 / scale))
                    .top(px((bounds.top - self.desktop.top) as f32 / scale))
                    .w(px(bounds.width as f32 / scale))
                    .h(px(bounds.height as f32 / scale))
                    .border_2()
                    .border_color(rgb(0xff5364))
                    .bg(rgba(0xff536420)),
            );
        }
        // This hint belongs to the temporary selection surface, never the recording.
        let hint = if !self.error.is_empty() {
            self.error.clone()
        } else if let Some(window) = &self.hover {
            window.title.clone()
        } else {
            text(
                "拖动选择区域 / 点击选择窗口 · Esc 取消",
                "Drag a region / click a window · Esc to cancel",
            )
            .to_owned()
        };
        root.child(
            div()
                .absolute()
                .top_4()
                .left_4()
                .px_3()
                .py_2()
                .rounded_md()
                .bg(rgb(0x202630))
                .text_color(rgb(0xffffff))
                .child(hint),
        )
    }
}

pub(super) fn drag_bounds(start: (i32, i32), end: (i32, i32)) -> CaptureBounds {
    CaptureBounds {
        left: start.0.min(end.0),
        top: start.1.min(end.1),
        width: start.0.abs_diff(end.0).min(i32::MAX as u32) as i32,
        height: start.1.abs_diff(end.1).min(i32::MAX as u32) as i32,
    }
}

pub(super) fn indicator_bounds(capture: CaptureBounds) -> Result<CaptureBounds> {
    Ok(CaptureBounds {
        left: capture
            .left
            .checked_sub(3)
            .ok_or_else(|| anyhow!("indicator coordinate overflow"))?,
        top: capture
            .top
            .checked_sub(35)
            .ok_or_else(|| anyhow!("indicator coordinate overflow"))?,
        width: capture
            .width
            .checked_add(6)
            .ok_or_else(|| anyhow!("indicator width overflow"))?,
        height: capture
            .height
            .checked_add(38)
            .ok_or_else(|| anyhow!("indicator height overflow"))?,
    })
}

pub(super) fn open_indicator(
    capture: CaptureBounds,
    target: RecordingTarget,
    label: &'static str,
    on_error: impl FnOnce(anyhow::Error, &mut App) + 'static,
    cx: &mut App,
) -> Result<AnyWindowHandle> {
    let bounds = indicator_bounds(capture)?;
    log::info!(target: "indicator", "Opening indicator: capture={capture:?}; outer={bounds:?}; target={target:?}");
    let handle = cx.open_window(options(bounds, target, false)?, |_, cx| {
        cx.new(|_| Indicator { label })
    })?;
    log::info!(target: "indicator", "Indicator created; applying native placement");
    if let Err(error) = handle.update(cx, |_, window, cx| {
        place_overlay(window, bounds, true, cx, on_error)
    })? {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
        return Err(error);
    }
    log::info!(target: "indicator", "Indicator placement configured");
    Ok(handle.into())
}

struct Indicator {
    label: &'static str,
}
impl Render for Indicator {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let scale = window.scale_factor();
        div()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h(px(28. / scale))
                    .px_2()
                    .bg(rgb(0xd63d50))
                    .text_color(rgb(0xffffff))
                    .text_size(px(12. / scale))
                    .child(self.label),
            )
            .child(
                div()
                    .absolute()
                    .top(px(32. / scale))
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .border(px(3. / scale))
                    .border_color(rgb(0xff5364)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{drag_bounds, indicator_bounds};
    #[test]
    fn reverse_drag_across_negative_desktop_coordinates() {
        let b = drag_bounds((150, 60), (-320, -90));
        assert_eq!((b.left, b.top, b.width, b.height), (-320, -90, 470, 150));
        assert!(drag_bounds((0, 0), (15, 40)).validate().is_err());
    }
    #[test]
    fn indicator_pixels_stay_outside_recorded_bounds() {
        let b = drag_bounds((-640, 120), (640, 840));
        let outer = indicator_bounds(b).unwrap();
        assert_eq!((b.left - outer.left, b.top - outer.top), (3, 35));
        assert_eq!((outer.width, outer.height), (b.width + 6, b.height + 38));
    }
}
