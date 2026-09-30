# 拾屏（ShiPing）

[English](README.md)

离线优先的桌面录屏工具。选择屏幕、可见窗口或区域，保存为本地 MP4 或 GIF，无需账号。

![拾屏录制控制条](images/shiping_01.jpg)

拾屏面向产品演示、操作教程、会议讲解和问题复现。紧凑控制条把录制状态和真正需要的操作集中在一起，不让设置打断表达。

![拾屏首选项](images/shiping_02.jpg)

- 录制单个屏幕、可见窗口或固定桌面区域。
- 保存为 MP4，可独立控制系统声音和麦克风；也可输出无声 GIF 动图。
- 支持暂停和继续，暂停期间录制时间线同步冻结。
- 可选自动、720p、1080p 或原始分辨率。
- MP4 支持 30/60 FPS，GIF 支持 10/20 FPS。
- 可录入鼠标指针，并可选择突出鼠标点击。
- 可设置开始倒计时、保存目录、托盘行为和全局快捷键。
- 录制窗口或区域时持续显示录制边界，直到停止或取消。

> **当前状态：** 本次迁移已在当前 Windows 设备通过短时 MP4 录制检查。macOS 和 Linux 后端已实现并保留 CI 配置，但独立仓库中的构建和真实设备录制尚未重新验证。

## 下载与反馈

- [Microsoft Store](https://apps.microsoft.com/detail/9NPZVCKN8H5Z)
- [GitHub 发布](https://github.com/dripai/ShiPing/releases)
- [反馈问题](https://github.com/dripai/ShiPing/issues)
- [隐私政策](PRIVACY.md)

## 本地构建

工作区默认运行 `apps/shiping`，当前构建配置使用 Rust 1.96.0、Slint 1.17.0。各平台依赖见[架构说明](apps/shiping/ARCHITECTURE.md)。

```bash
./start.sh dev
./start.sh build
```

脚本需要 Bash，Windows 使用 Git Bash。也可以直接使用 Cargo：

```bash
cargo check --locked --package shiping
cargo test --locked --workspace
cargo build --release --locked --package shiping --bin ShiPing
```

版本标签只发布拾屏：Windows x64、Linux x64、macOS ARM64 和 macOS x64。macOS 归档仅使用 ad-hoc 签名，尚未完成 Apple 公证；Linux 归档依赖架构说明列出的系统组件。

## 项目目录

- `apps/shiping`：界面、录制核心、平台后端、翻译和图标。
- `crates/shi-foundation`、`crates/shi-ui`：本仓库独立维护的内部模块。
- `packaging/shiping`：原有 Microsoft Store 身份和清单。
- `tools`：平台打包脚本。

版本从 `0.1.34` 延续，配置路径和打包身份保持不变。迁移细节见[迁移记录](MIGRATION.md)。
