mod windowing;

#[cfg(any(target_os = "macos", test))]
mod coordinates;

pub(crate) use windowing::{native_window_id, place_overlay, pointer_position};

#[cfg(target_os = "macos")]
pub(crate) use macos::{overlay_geometry, selection_surfaces};

#[cfg(target_os = "linux")]
mod ffmpeg;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
mod macos_writer;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
mod unsupported;

use crate::ports::{DesktopIntegration, RecordingBackend, TargetSelection};

pub(crate) fn desktop_integration() -> &'static dyn DesktopIntegration {
    #[cfg(target_os = "windows")]
    {
        &windows::DESKTOP_INTEGRATION
    }
    #[cfg(target_os = "macos")]
    {
        &macos::DESKTOP_INTEGRATION
    }
    #[cfg(target_os = "linux")]
    {
        &linux::DESKTOP_INTEGRATION
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        &unsupported::DESKTOP_INTEGRATION
    }
}

pub(crate) fn recording_backend() -> &'static dyn RecordingBackend {
    #[cfg(target_os = "windows")]
    {
        &windows::RECORDING_BACKEND
    }
    #[cfg(target_os = "macos")]
    {
        &macos::RECORDING_BACKEND
    }
    #[cfg(target_os = "linux")]
    {
        &linux::RECORDING_BACKEND
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        &unsupported::RECORDING_BACKEND
    }
}

pub(crate) fn target_selection() -> &'static dyn TargetSelection {
    #[cfg(target_os = "windows")]
    {
        windows::target_selection()
    }
    #[cfg(target_os = "macos")]
    {
        &macos::TARGET_SELECTION
    }
    #[cfg(target_os = "linux")]
    {
        &linux::TARGET_SELECTION
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        &unsupported::TARGET_SELECTION
    }
}
