# ShiPing 架构

项目为单个 Cargo package，源码统一位于 `src/`。当前 UI 迁移范围为 Windows 和 macOS；Linux 源码和打包材料保留，但不在当前构建、验证与发布矩阵中。

## 代码边界

| 位置 | 职责 |
| --- | --- |
| `src/main.rs`、`src/config.rs`、`src/i18n.rs` | 入口、JSON 配置、中文/英文文本 |
| `src/logging.rs` | 文件日志、GPUI 日志接收、Rust panic 堆栈 |
| `src/ui/controller.rs` | 录制状态、事件轮询与 GPUI 窗口生命周期 |
| `src/ui/main_panel.rs` | 主界面横向工具条、计时及目标摘要、圆形按钮、声音开关与原生菜单 |
| `src/ui/preferences.rs`、`src/ui/theme.rs` | 常规/录制/快捷键设置、主题及系统外观通知 |
| `src/ui/overlay.rs`、`src/ui/tray.rs`、`src/ui/hotkeys.rs` | 目标选择、录制边界、原生托盘、全局快捷键 |
| `src/domain.rs`、`src/ports.rs` | 目标与坐标模型、采集/编码/桌面集成端口 |
| `src/application/recording_service.rs` | 录制线程、暂停/继续、媒体时间轴、运行时选项和输出提交 |
| `src/platform/windows/` | Win32 枚举、GDI 视频、WASAPI 音频、Media Foundation MP4 |
| `src/platform/macos.rs`、`src/platform/macos_writer.rs` | ScreenCaptureKit 视频/音频、AVFoundation H.264/AAC MP4 |
| `src/platform/windowing.rs`、`src/platform/coordinates.rs` | GPUI 原生句柄、覆盖窗口输入穿透、显示器坐标转换 |
| `src/output.rs` | GIF 编码、临时输出和最终文件生命周期 |

业务录制核心通过端口调用平台能力，不直接持有 GPUI 窗口或平台音视频对象。目标枚举端口接收 GPUI `App`，用于 macOS 显示器映射；窗口原生处理集中在平台层。

## UI 与配置

- 使用 `gpui-kit =0.7.3`，在 Cargo 中别名为 `gpui`，底层锁定 `gpui-pre 0.3.9` 和 `gpui-component 0.7.3`。没有 Slint 运行时或编译步骤。
- 常规按钮、菜单、输入框、设置页侧栏和开关使用组件库现有实现；仅录制目标选择与录制边界自行绘制。设置窗口采用 96 逻辑像素宽的固定左侧导航，右侧表单可滚动，底部操作栏固定。
- 主界面参考原版横向工具条，固定为 680 × 94（逻辑像素），通过 GPUI 隐藏标题栏，使用不透明主题背景与 GPUI Kit 标准 Root。内容背景铺满客户区，移除额外的 4 像素外留白、内层圆角和描边，避免与系统外框叠层。左侧为取景框标识、大计时器和单行目标/参数摘要，右侧为范围、画质、帧率、系统声音、麦克风和录制的圆形控件。声音图标随状态切换：开启为 `Volume2` / `Mic`，关闭为 `VolumeOff` / `MicOff`。范围与摘要共用原生目标菜单；格式、倒计时、鼠标选项、设置及窗口操作收进右键菜单。录制中范围切换为暂停/继续，主操作切换为停止；初始化与保存期间禁用声音和鼠标调整。快捷键在按钮悬停说明中显示，完成后摘要保留已保存文件入口，错误和警告优先显示。
- 主窗口图标及计时区使用 GPUI 官方窗口拖动接口，Windows 标记 `WindowControlArea::Drag`，macOS 调用 `Window::start_window_move`。设置窗口的标题栏、尺寸与排版保持现状。
- 主界面通过 `NativeMenu` 显示 Windows/macOS 系统菜单，支持越过工具条边界；窗口内绘制的 `PopupMenu` 仅保留在设置页。右键打开操作菜单，首项为设置；菜单选择通过 GPUI `Action`、工具条 `FocusHandle` 和 `on_action` 接回现有业务操作，打开及选择动作写入日志。失焦关闭、键盘导航及屏幕边缘定位由系统菜单处理，实际交互仍待用户验证。
- 主窗口关闭时最小化，托盘可以恢复、控制录制、打开设置或退出。录制退出会等待文件完成。
- 主题保存为 `theme: "system" | "light" | "dark"`，旧配置缺少该字段时默认跟随系统。保存后应用；取消不应用草稿。系统主题变化通过 GPUI 窗口外观通知同步。
- Windows 原生标题栏遵循系统外观；GPUI 的应用外观覆盖只在 macOS 实现。应用内容区使用所选主题。
- 设置打开期间暂停全局快捷键注册，关闭或保存时恢复；注册冲突和配置写入失败显式报错。
- 配置路径与包身份保留：Windows 为 `%APPDATA%/ShiPing/config.json`；macOS 为 `~/Library/Application Support/ShiPing/config.json`。
- 日志写入同一配置目录下的 `logs/`，每次启动使用独立文件。记录启动、目标选择、窗口切换、录制状态和错误；Rust panic 会强制捕获堆栈并同步日志。日志初始化失败时明确报错退出。

## 平台限制

- Windows 保留 GDI、WASAPI 和 Media Foundation 管线；鼠标点击高亮由 Windows 后端实现。
- macOS 使用 ScreenCaptureKit 8.0.1 的 macOS 15 API。构建需要 Xcode、Swift 与 Apple SDK，部署目标与 `Info.plist` 均为 15.0。屏幕录制和麦克风受系统权限控制；点击高亮尚未实现，在 UI 中禁用。
- macOS 将显示器物理像素保存在应用内部的独立坐标空间，覆盖窗口按显示器 ID 转回该屏幕内的逻辑点。区域选择限于单个显示器；混合 Retina 缩放与跨屏窗口仍待实机验证。
- Windows 录制边界使用 Win32 layered/input-transparent 工具窗口；macOS 使用 `NSWindow.setIgnoresMouseEvents`。输入穿透、屏幕边缘及缩放行为仍待完整实测。
- Windows 原生覆盖窗口定位通过 GPUI 主线程任务执行，原生调用期间不持有 GPUI `App` 借用，避免跨显示器的同步回调重入。定位以实际客户区为准：移到目标显示器后测量非客户区边距、换算外框、重新定位并核对；指示窗口对齐后才显示，避免把外框偏移带入绘制区域。副屏区域选择完成及边框显示已由用户确认正常；边框进入视频的修复待录制复测，其他交互仍需验证。
- Linux 的 Portal/PipeWire/FFmpeg 实现和包文件仍在仓库中；当前 GPUI 迁移不对其可用性作保证。

## 构建与分发

```bash
./start.sh       # dev：cargo run --bin ShiPing
./start.sh build # release：cargo build --release --bin ShiPing
```

Windows 使用 Rust MSVC 工具链、MSVC 链接工具和 Windows SDK；脚本入口需要 Git Bash。构建产物默认位于本仓库 `target/`。

CI 配置 Windows 和 macOS 检查/测试，版本标签发布矩阵为 Windows x64、macOS ARM64、macOS x64。矩阵配置不代表已经运行成功。Windows Store 打包使用 `packaging/windows`；macOS `.app.zip` 使用 `packaging/macos`，只做 ad-hoc 签名，未做 Developer ID 签名或 Apple 公证。

## 验证与官方依据

本次实际验证、未验证项和原生适配原因见 [GPUI 迁移说明](GPUI-MIGRATION.md)。此前目录调整与独立仓库历史见 [迁移记录](../MIGRATION.md)。

- [GPUI Kit 0.7.3](https://docs.rs/gpui-kit/0.7.3/gpui_kit/)
- [GPUI Component 0.7.3](https://docs.rs/gpui-component/0.7.3/gpui_component/)
- [ScreenCaptureKit 8.0.1](https://docs.rs/screencapturekit/8.0.1/screencapturekit/)
- [Apple ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
- [Apple AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter)
- [Windows WASAPI loopback](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)
- [Windows Media Foundation Sink Writer](https://learn.microsoft.com/en-us/windows/win32/medfound/sink-writer)
