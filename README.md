# ShiPing

[简体中文](README.zh-CN.md)

An offline-first desktop screen recorder. Choose a display, visible window, or region and save a local MP4 or GIF. No account is required.

![ShiPing recording toolbar](images/shiping_01.jpg)

ShiPing is designed for product walkthroughs, tutorials, meeting demonstrations, and reproducible bug reports. Its compact toolbar keeps the active recording state and the controls you need in one place.

![ShiPing preferences](images/shiping_02.jpg)

- Record one screen, a visible window, or a fixed desktop region.
- Save as MP4 with optional system audio and microphone input, or as a silent animated GIF.
- Pause and resume without advancing the recording timeline.
- Choose automatic, 720p, 1080p, or original resolution.
- Use 30/60 FPS for MP4 and 10/20 FPS for GIF.
- Include the pointer and optionally highlight mouse clicks.
- Configure the countdown, save folder, tray behavior, and global shortcuts.
- Keep a visible recording boundary around the selected window or region until recording stops.

> **Current status:** Windows recording passed a short MP4 check during this migration. macOS and Linux backends are implemented and have CI configuration; builds and real-device recording have not been revalidated in this independent repository.

## Download and support

- [Microsoft Store](https://apps.microsoft.com/detail/9NPZVCKN8H5Z)
- [GitHub releases](https://github.com/dripai/ShiPing/releases)
- [Report a problem](https://github.com/dripai/ShiPing/issues)
- [Privacy policy](PRIVACY.en.md)

## Build locally

The workspace defaults to `apps/shiping`; Rust 1.96.0 and Slint 1.17.0 are used by the current build configuration. Install the platform dependencies described in [the architecture notes](apps/shiping/ARCHITECTURE.md).

```bash
./start.sh dev
./start.sh build
```

The script requires Bash (Git Bash on Windows). Standard Cargo commands also work:

```bash
cargo check --locked --package shiping
cargo test --locked --workspace
cargo build --release --locked --package shiping --bin ShiPing
```

Version tags publish only ShiPing: Windows x64, Linux x64, macOS ARM64, and macOS x64. macOS archives use an ad-hoc signature and have no Apple notarization; Linux archives require the runtime dependencies listed in the architecture notes.

## Project layout

- `apps/shiping`: UI, recording core, platform backends, translations, and icons.
- `crates/shi-foundation`, `crates/shi-ui`: internal modules maintained in this repository.
- `packaging/shiping`: existing Microsoft Store identity and manifest.
- `tools`: platform packaging scripts.

The version continues from `0.1.34`. Configuration paths and package identity are preserved. See [the migration record](MIGRATION.md).
