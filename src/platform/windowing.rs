use crate::domain::{Bounds, WindowId};
use anyhow::Result;
use gpui::{App, Pixels, Point, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub(crate) fn native_window_id(window: &Window) -> Option<WindowId> {
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => {
            Some(WindowId::from_platform_value(handle.hwnd.get() as u64))
        }
        RawWindowHandle::Xlib(handle) => Some(WindowId::from_platform_value(handle.window as u64)),
        RawWindowHandle::Xcb(handle) => {
            Some(WindowId::from_platform_value(handle.window.get() as u64))
        }
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(handle) => {
            let view = unsafe { &*handle.ns_view.as_ptr().cast::<objc2_app_kit::NSView>() };
            view.window()
                .map(|window| WindowId::from_platform_value(window.windowNumber() as u64))
        }
        _ => None,
    }
}

/// GPUI 0.3.9 Windows has no set_input_region implementation. Keep the only
/// native extension here: physical placement and a layered, input-transparent
/// tool window. See docs/GPUI-MIGRATION.md for the official API evidence.
pub(crate) fn place_overlay(
    window: &Window,
    bounds: Bounds,
    visual_only: bool,
    cx: &mut App,
    on_error: impl FnOnce(anyhow::Error, &mut App) + 'static,
) -> Result<()> {
    log::info!(target: "windowing", "Place overlay: bounds={bounds:?}; visual_only={visual_only}");
    #[cfg(target_os = "windows")]
    {
        use anyhow::anyhow;
        use windows::Win32::Foundation::HWND;
        let RawWindowHandle::Win32(handle) = HasWindowHandle::window_handle(window)?.as_raw()
        else {
            return Err(anyhow!("GPUI window has no Win32 handle"));
        };
        let hwnd = HWND(handle.hwnd.get() as *mut _);
        let handle = window.window_handle();
        let mut app = cx.to_async();
        // SetWindowPos synchronously dispatches move/DPI/display notifications.
        // GPUI 0.3.9's display callback borrows App directly. Run native calls
        // outside handle.update/App::update, as GPUI's Windows backend does for
        // its own move/resize operations. Otherwise moving to another display
        // aborts with `RefCell already borrowed` across the Win32 callback.
        cx.foreground_executor()
            .spawn(async move {
                // The overlay may have been canceled before this task ran.
                if handle.update(&mut app, |_, _, _| ()).is_err() {
                    log::info!(target: "windowing", "Skipping placement of a closed overlay");
                    return;
                }
                // Do not put this call inside app.update or handle.update.
                if let Err(error) = place_windows_overlay(hwnd, bounds, visual_only) {
                    log::error!(target: "windowing", "Overlay placement failed: {error:#}");
                    app.update(|cx| on_error(error, cx));
                }
            })
            .detach();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (bounds, cx, on_error);
        if visual_only {
            #[cfg(target_os = "macos")]
            {
                use objc2_app_kit::NSView;
                let RawWindowHandle::AppKit(handle) =
                    HasWindowHandle::window_handle(window)?.as_raw()
                else {
                    anyhow::bail!("GPUI window has no AppKit handle");
                };
                let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
                let native = view
                    .window()
                    .ok_or_else(|| anyhow::anyhow!("NSView has no window"))?;
                native.setIgnoresMouseEvents(true);
            }
            #[cfg(target_os = "linux")]
            window.set_input_region(Some(&[]));
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn place_windows_overlay(
    hwnd: windows::Win32::Foundation::HWND,
    bounds: Bounds,
    visual_only: bool,
) -> Result<()> {
    use windows::Win32::{
        Foundation::{GetLastError, SetLastError, WIN32_ERROR},
        UI::WindowsAndMessaging::*,
    };
    unsafe {
        let mut style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        style |= WS_EX_TOOLWINDOW.0;
        if visual_only {
            style |= WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0;
        }
        log::info!(target: "windowing", "Set overlay style: hwnd={hwnd:?}; exstyle={style:#x}");
        SetLastError(WIN32_ERROR(0));
        if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize) == 0 && GetLastError().0 != 0 {
            return Err(windows::core::Error::from_thread().into());
        }
        if visual_only {
            log::info!(target: "windowing", "Set layered window attributes");
            SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                LWA_ALPHA,
            )?;
        }
        // First move to the destination display and apply the new frame style.
        // WM_DPICHANGED can synchronously change the non-client margins here.
        log::info!(target: "windowing", "Move overlay to destination display");
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            bounds.left,
            bounds.top,
            bounds.width,
            bounds.height,
            SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )?;
        let (outer, client) = windows_overlay_geometry(hwnd)?;
        let adjusted = window_bounds_for_client(bounds, outer, client)?;
        log::info!(target: "windowing", "Align overlay client: requested={bounds:?}; outer={outer:?}; client={client:?}; adjusted_outer={adjusted:?}");
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            adjusted.left,
            adjusted.top,
            adjusted.width,
            adjusted.height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
        let (outer, client) = windows_overlay_geometry(hwnd)?;
        anyhow::ensure!(
            client == bounds,
            "Overlay client placement mismatch: requested={bounds:?}; actual={client:?}; outer={outer:?}"
        );
        log::info!(target: "windowing", "Overlay native placement complete: outer={outer:?}; client={client:?}");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_overlay_geometry(hwnd: windows::Win32::Foundation::HWND) -> Result<(Bounds, Bounds)> {
    use windows::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
    };
    let mut outer = RECT::default();
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        GetWindowRect(hwnd, &mut outer)?;
        GetClientRect(hwnd, &mut client)?;
        ClientToScreen(hwnd, &mut origin).ok()?;
    }
    Ok((
        Bounds {
            left: outer.left,
            top: outer.top,
            width: outer.right - outer.left,
            height: outer.bottom - outer.top,
        },
        Bounds {
            left: origin.x,
            top: origin.y,
            width: client.right - client.left,
            height: client.bottom - client.top,
        },
    ))
}

/// GPUI 0.3.9 lays out overlays in the client area, but SetWindowPos sizes the
/// entire native window. Use measured margins (including asymmetric top/bottom
/// margins) rather than assuming a borderless popup or a fixed DPI-dependent inset.
#[cfg(any(target_os = "windows", test))]
fn window_bounds_for_client(desired: Bounds, outer: Bounds, client: Bounds) -> Result<Bounds> {
    let coordinate = |value: i64| {
        i32::try_from(value).map_err(|_| anyhow::anyhow!("Overlay window coordinate overflow"))
    };
    let adjusted = Bounds {
        left: coordinate(i64::from(desired.left) + i64::from(outer.left) - i64::from(client.left))?,
        top: coordinate(i64::from(desired.top) + i64::from(outer.top) - i64::from(client.top))?,
        width: coordinate(
            i64::from(desired.width) + i64::from(outer.width) - i64::from(client.width),
        )?,
        height: coordinate(
            i64::from(desired.height) + i64::from(outer.height) - i64::from(client.height),
        )?,
    };
    anyhow::ensure!(
        desired.width > 0 && desired.height > 0 && adjusted.width > 0 && adjusted.height > 0,
        "Invalid overlay client/window dimensions"
    );
    Ok(adjusted)
}

pub(crate) fn pointer_position(
    window: &Window,
    local: Point<Pixels>,
    desktop: Bounds,
) -> Result<(i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        let _ = (window, local, desktop);
        let mut point = windows::Win32::Foundation::POINT::default();
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point)?;
        }
        Ok((point.x, point.y))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let scale = window.scale_factor();
        Ok((
            desktop.left + (f32::from(local.x) * scale).round() as i32,
            desktop.top + (f32::from(local.y) * scale).round() as i32,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_client_alignment_accounts_for_frame_and_display_scale() {
        // Regression: the 425x280 capture at (-1416, 275) recorded the red
        // border because an 8px left/right/bottom native frame shrank the client.
        let desired = Bounds {
            left: -1419,
            top: 240,
            width: 431,
            height: 318,
        };
        for inset in [0, 8, 10, 12, 16] {
            let client = Bounds {
                left: desired.left + inset,
                top: desired.top,
                width: desired.width - 2 * inset,
                height: desired.height - inset,
            };
            let adjusted = window_bounds_for_client(desired, desired, client).unwrap();
            // Reapply the actual asymmetric margins: the drawable rectangle
            // must equal the requested rectangle at every tested scale.
            let actual_client = Bounds {
                left: adjusted.left + inset,
                top: adjusted.top,
                width: adjusted.width - 2 * inset,
                height: adjusted.height - inset,
            };
            assert_eq!(actual_client, desired);
        }
    }

    #[test]
    fn overlay_alignment_rejects_coordinate_overflow() {
        let desired = Bounds {
            left: i32::MIN,
            top: 0,
            width: 431,
            height: 318,
        };
        let client = Bounds {
            left: desired.left + 8,
            width: desired.width - 16,
            height: desired.height - 8,
            ..desired
        };
        assert!(window_bounds_for_client(desired, desired, client).is_err());
    }
}
