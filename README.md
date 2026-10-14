# ShiPing

[简体中文](README.zh-CN.md)

An offline-first desktop screen recorder. Choose a display, visible window, or region and save a local MP4 or GIF. No account is required.

- Record one screen, a visible window, or a fixed desktop region.
- Save as MP4 with optional system audio and microphone input, or as a silent animated GIF.
- Pause and resume without advancing the recording timeline.
- Choose automatic, 720p, 1080p, or original resolution.
- Use 30/60 FPS for MP4 and 10/20 FPS for GIF.
- Include the pointer and optionally highlight mouse clicks.
- Configure the countdown, save folder, tray behavior, and global shortcuts.
- Choose System, Light, or Dark appearance. Preferences are grouped into General, Recording, and Shortcuts.
- Keep a visible recording boundary around the selected window or region until recording stops.

> **Current status:** The UI now uses GPUI, targeting Windows and macOS. Windows compilation, unit tests, and Clippy pass. Full macOS builds and recording interactions on both platforms remain unverified. See the [GPUI migration notes](docs/GPUI-MIGRATION.md) for exact versions and validation limits.

## Download and support

- [Microsoft Store](https://apps.microsoft.com/detail/9NPZVCKN8H5Z)
- [GitHub releases](https://github.com/dripai/ShiPing/releases)
- [Report a problem](https://github.com/dripai/ShiPing/issues)
- [Privacy policy](PRIVACY.en.md)

## Build locally

This repository contains a single Cargo package, with source code in `src/`. The current build configuration uses Rust 1.96.0 and GPUI Kit 0.7.3 (GPUI 0.3.9). Install the platform dependencies described in [the architecture notes](docs/ARCHITECTURE.md).

```bash
./start.sh       # Defaults to dev
./start.sh build
```

The script requires Bash (Git Bash on Windows). Standard Cargo commands also work:

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked --bin ShiPing
```

The version-tag release matrix includes Windows x64, macOS ARM64, and macOS x64. macOS requires Xcode/Swift and an Apple SDK, with a macOS 15.0 deployment target. Archives use an ad-hoc signature without Apple notarization. Linux sources and packaging are retained but are outside this migration's build and validation scope.

## Project layout

- `src`: application entry point, configuration, internationalization, and recording models.
- `src/application`, `src/platform`: recording workflows and platform backends.
- `src/ui`: GPUI views, recording state control, target selection, preferences, and tray integration.
- `assets`: application icons. English/Chinese UI text is selected by `src/i18n.rs`.
- `packaging/windows`, `packaging/macos`, `packaging/linux`: platform packaging materials.
- `tools`: platform packaging scripts.
- `docs/ARCHITECTURE.md`: architecture and platform limitations.

The current version is `0.2.0`, continuing from `0.1.34`. Configuration paths and package identity are preserved. See [the migration record](MIGRATION.md).
