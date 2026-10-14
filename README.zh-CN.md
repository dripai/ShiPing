# 拾屏（ShiPing）

[English](README.md)

离线优先的桌面录屏工具。选择屏幕、可见窗口或区域，保存为本地 MP4 或 GIF，无需账号。

- 录制单个屏幕、可见窗口或固定桌面区域。
- 保存为 MP4，可独立控制系统声音和麦克风；也可输出无声 GIF 动图。
- 支持暂停和继续，暂停期间录制时间线同步冻结。
- 可选自动、720p、1080p 或原始分辨率。
- MP4 支持 30/60 FPS，GIF 支持 10/20 FPS。
- 可录入鼠标指针，并可选择突出鼠标点击。
- 可设置开始倒计时、保存目录、托盘行为和全局快捷键。
- 支持跟随系统、浅色和深色主题；设置分为常规、录制和快捷键。
- 录制窗口或区域时持续显示录制边界，直到停止或取消。

> **当前状态：** UI 已迁移到 GPUI，当前目标为 Windows 和 macOS。Windows 已通过编译、单元测试和 Clippy；macOS 完整构建与两平台录制交互仍待验证。具体版本和验证边界见 [GPUI 迁移说明](docs/GPUI-MIGRATION.md)。

## 下载与反馈

- [Microsoft Store](https://apps.microsoft.com/detail/9NPZVCKN8H5Z)
- [GitHub 发布](https://github.com/dripai/ShiPing/releases)
- [反馈问题](https://github.com/dripai/ShiPing/issues)
- [隐私政策](PRIVACY.md)

## 本地构建

仓库采用单个 Cargo package，源码统一位于 `src/`。当前构建配置使用 Rust 1.96.0、GPUI Kit 0.7.3（GPUI 0.3.9）。各平台依赖见[架构说明](docs/ARCHITECTURE.md)。

```bash
./start.sh       # 默认执行 dev
./start.sh build
```

脚本需要 Bash，Windows 使用 Git Bash。也可以直接使用 Cargo：

```bash
cargo check --locked
cargo test --locked
cargo build --release --locked --bin ShiPing
```

版本标签的发布矩阵为 Windows x64、macOS ARM64 和 macOS x64。macOS 需要 Xcode/Swift 和 Apple SDK，部署目标为 macOS 15.0；归档仅使用 ad-hoc 签名，尚未完成 Apple 公证。Linux 源码和打包材料保留，本次不构建或验证。

## 项目目录

- `src`：应用入口、配置、国际化与录制领域模型。
- `src/application`、`src/platform`：录制业务流程和平台后端。
- `src/ui`：GPUI 视图、录制状态控制、目标选择、设置和托盘集成。
- `assets`：应用图标；中英文界面文本由 `src/i18n.rs` 选择。
- `packaging/windows`、`packaging/macos`、`packaging/linux`：各平台打包材料。
- `tools`：平台打包脚本。
- `docs/ARCHITECTURE.md`：架构与平台限制。

当前版本为 `0.2.0`，从 `0.1.34` 延续，配置路径和打包身份保持不变。迁移细节见[迁移记录](MIGRATION.md)。
