# GPUI 迁移与验证

2026-10-14：UI 已从 Slint 迁移到 GPUI，范围为 Windows 和 macOS。录制核心、MP4/GIF 写入管线、配置路径及应用包身份保留；Linux 文件保留，排除当前 CI/发布构建矩阵。

## 依赖与官方能力

| 依赖 | 锁定版本 | 使用方式 |
| --- | --- | --- |
| `gpui-kit`（别名 `gpui`） | 0.7.3 | 初始化、`assets::AllAssets` 图标资源、平台入口与窗口 Root |
| `gpui-pre` | 0.3.9 | 窗口、渲染、输入、定时任务、外观通知 |
| `gpui-component` | 0.7.3 | `Button`、`Toggle`、`Input/InputState`、主界面 `NativeMenu`、设置页 `DropdownMenu/PopupMenu/PopupMenuItem`、`Sidebar/SidebarMenu/SidebarMenuItem`、`Form/Field`、`Switch`、`Theme` |
| `tray-icon` | 0.21.2 | 原生托盘及菜单 |
| `raw-window-handle` | 0.6.2 | GPUI 原生窗口句柄 |
| `objc2-app-kit` | 0.3.2 | macOS `NSView.window`、`NSWindow.windowNumber/setIgnoresMouseEvents` |
| `log` | 0.4.33 | 接收应用和 GPUI 的日志，写入本地文件 |

以上能力已核对当前锁文件和 Cargo registry 对应版本源码。GPUI Kit 是本项目采用的组件入口；底层实际使用 `gpui-pre`，没有直接依赖另一套 GPUI 引擎。

## 项目实现

- 主界面、设置页、目标选择、录制边界、托盘与全局快捷键均接入 GPUI 事件循环。已删除 Slint 依赖、构建代码、视图、旧翻译目录和相关窗口管理代码。
- 设置页使用组件库 `Sidebar`、`SidebarMenu` 和 `SidebarMenuItem`，左侧为 96 逻辑像素宽、不可折叠的文字导航，分为常规、录制、快捷键；选中项和悬停状态使用组件主题。窗口默认 560 × 420，最小 520 × 360（逻辑像素）。右侧不重复页面标题，表单使用行内标签；选择按钮宽度随内容区伸缩，上限 180，快捷键按钮与清除按钮共同适应剩余宽度。保存目录、浏览和打开按钮位于同一行，录制页开关分两列。仅右侧内容可滚动，恢复默认/取消/保存固定在整窗底部，恢复默认只修改草稿。已移除顶部 `TabBar/Tab`；排版、中英文和缩放下的实际显示与交互仍由用户手动测试。
- 新增跟随系统、浅色、深色主题，默认跟随系统。`Theme::change` 切换内容区，`Window::observe_window_appearance` 和 `Theme::sync_system_appearance` 同步系统变化；保存后生效，取消保留原主题。
- GPUI `App::set_window_appearance` 在 macOS 可控制原生窗口外观，在 Windows 是空实现。因此 Windows 标题栏继续遵循系统外观。
- 保存设置前校验目录与快捷键；快捷键注册和配置落盘失败显式报错。设置窗口打开期间禁用主界面的配置修改与开始录制，避免草稿覆盖同时发生的更改。
- macOS 每个显示器单独创建选区覆盖窗口，按 ScreenCaptureKit 的物理像素和 GPUI 的显示器内逻辑点转换；区域不跨显示器。
- 主界面按仓库原版横向工具条图片重排，固定为 680 × 94（逻辑像素）：左侧为取景框图标、大计时器和单行摘要，中间为竖分隔线，右侧为范围、画质、帧率、系统声音、麦克风和录制的圆形控件及简短标签。颜色沿用已选主题；声音开启使用 `Volume2` / `Mic`，关闭使用内置的斜线图标 `VolumeOff` / `MicOff`，图标直接读取声音开关状态，悬停提示仍显示开/关。目标标题来自选择器已有的 `WindowCandidate.title`，没有新增系统查询或预览采集。
- 主界面的目标、画质、帧率、右键及二级菜单均使用 `NativeMenu::from(gpui::Menu).show`，由 Windows/macOS 的系统菜单绘制，可超出工具条窗口边界。范围按钮和摘要共用目标菜单，列出显示器、窗口和区域；画质及帧率各有独立选择菜单。右键打开操作菜单，“设置”为首项，另含保存目录、显示器刷新、格式、倒计时、鼠标选项、最小化和退出。录制中范围按钮切换为暂停/继续，录制按钮切换为停止；倒计时可取消，初始化时可请求停止，保存时主按钮禁用。声音开关始终位于工具条，初始化和保存期间禁用；GIF 和平台能力限制继续生效。
- 快捷键在对应按钮悬停说明中显示。完成后摘要改为可点击的文件名，悬停可见完整路径；错误和警告优先显示，长文字省略并可悬停查看完整内容。图标使用 GPUI Kit 原生 `AllAssets`，取景框标识由已有 `Scan` 图标和圆点组合。按钮、开关、菜单、提示、焦点及菜单关闭行为沿用原生组件。工具条的实际交互、中英文、主题和缩放显示待用户验证。

## 自定义范围与依据

原生 `TitleBar` 自带窗口控制按钮，无法直接复用为原图的一行配置工具条。主窗口使用 `WindowOptions::titlebar = None` 隐藏标题栏，`WindowBackgroundAppearance::Opaque` 和 GPUI Kit 标准 Root 提供主题背景。此前透明窗口中额外的 4 逻辑像素外留白、17 像素内层圆角与描边与系统外框叠在一起，用户截图出现明显白圈；现已移除，内容背景铺满客户区，窗口外形交给平台处理。不新增边框或阴影系统 API，设置窗口不变。仅图标和计时区允许拖动：Windows 使用 `InteractiveElement::window_control_area(WindowControlArea::Drag)`；macOS 按对应版本 `window_shadow` 示例在按下时调用 `Window::start_window_move`，并设置 `app_owns_titlebar_drag`。不自行计算窗口拖动坐标。

`PopupMenu` 是窗口内部绘制的组件，不能用于 94 像素高的工具条菜单；此前用户截图已确认菜单被裁切。主界面已移除该实现，改用当前版本已提供的 `NativeMenu`。菜单通过 GPUI `Action` 分发到工具条的原生 `FocusHandle` 与 `on_action`，再调用已有录制目标、设置和窗口操作；不另建菜单窗口，不手写系统菜单 API。右键按下/松开均消费事件，避免拖动区继续触发系统标题栏菜单。菜单打开及选择动作写入 `ui-menu` 日志。设置页的窗口内菜单保持现有实现。

目标选择是应用特有的窗口命中/区域拖动行为，组件库没有录屏目标选择组件；只在 GPUI 覆盖窗口中绘制选择边界并处理输入，普通菜单、标签和控件继续使用组件库。

GPUI 0.3.9 的 `WindowKind::PopUp` 在 Windows 使用置顶工具窗口。`PlatformWindow::set_input_region` 默认为空实现，Windows/macOS 后端没有覆盖，不能据此宣称支持鼠标穿透：

- Windows 在 `src/platform/windowing.rs` 设置 `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE`，并使用 `SetLayeredWindowAttributes`、`SetWindowPos`。Microsoft 文档说明 layered 窗口配合 `WS_EX_TRANSPARENT` 可把鼠标事件传给下面的窗口；其与 GPUI 渲染的组合仍待交互实测。
- macOS 使用 `NSWindow.setIgnoresMouseEvents(true)`。显示器坐标依据 GPUI macOS 后端按 `display_id` 选择 `NSScreen`、再将显示器内逻辑边界转换成原生窗口坐标的实际实现。

录制边界按采集矩形外布局，坐标单元测试覆盖边框和标签外扩、负坐标拖动及 Retina 比例换算。副屏区域选择和边框显示已由用户确认正常；随后用户提供的录制文件暴露了 Windows 原生窗口外框与客户区的定位偏差，见下文。跨屏窗口移动与屏幕边缘行为仍需实机验证。

## 区域选择崩溃修复与日志

用户复现“拖选区域后松开鼠标退出”，新增日志捕获到调用链：`SetWindowPos → WindowsWindowInner::set_display → AsyncApp::update → RefCell already borrowed`。当时录制边框从默认显示器移动到副屏，Windows 同步触发显示器变更回调，而 GPUI `App` 仍在 `handle.update` 内被借用，导致 panic 穿过原生窗口回调并终止进程。

修复沿用 GPUI Windows 后端的主线程任务方式：先读取句柄，将原生样式和定位操作放到 `ForegroundExecutor::spawn` 任务中执行；执行前检查窗口仍存在，原生调用期间不进入 `App::update` 或 `handle.update`。异步失败会返回界面错误并关闭相关覆盖窗口。用户已在同一副屏确认不再退出，区域边框显示正常。

日志位置：

- Windows：`%APPDATA%/ShiPing/logs/`。
- macOS：`~/Library/Application Support/ShiPing/logs/`。

每次启动创建 `shiping-<Unix毫秒>-<进程ID>.log`，每行为一条 UTF-8 JSON，包含时间、进程/线程、级别、模块、代码位置和消息。记录启动、选区按下/松开、窗口创建/定位/关闭、录制状态和错误；接收 GPUI 的 Info/Warn/Error 日志，不逐帧或逐次鼠标移动写日志。文件暂不自动清理。

Rust panic 使用 `std::panic::set_hook` 与 `Backtrace::force_capture` 写入位置、消息和堆栈，并在原有 panic 处理前同步日志。它不拦截所有原生异常或进程强制结束；这些情况仍需结合最后一条操作日志和系统错误记录判断。初始化日志失败会明确报错退出，不静默丢弃启动日志。

## Windows 录制边框进入视频

用户提供的 `Recording_20261014_160755.mp4`（424 × 280）抽帧可见左右和底部的红线。日志中的采集源为 `(-1416, 275, 425, 280)`，边框窗口请求为 `(-1419, 240, 431, 318)`。原有测试只验证外扩数值，没有覆盖原生客户区。GPUI 0.3.9 的 Windows 后端以客户区尺寸渲染，并在自己的窗口定位中计算非客户区偏移；项目的 `SetWindowPos` 却直接设置整个窗口外框，导致绘制区域内缩。

修复仍在原生主线程任务中执行：先移动到目标显示器、应用样式，再通过 `GetWindowRect`、`GetClientRect`、`ClientToScreen` 测量当前客户区和外框，将请求的绘制矩形换算为原生外框后定位。最后读取客户区核对位置和尺寸；不一致时明确报错。偏移取自实际窗口，不写死边框宽度，也不假设上下边距对称。Windows 指示窗口在客户区对齐后才显示，日志同时记录请求矩形、实际外框、客户区和调整后的外框。回归测试覆盖本次副屏负坐标、不同边距及溢出；新版本录制效果待用户复测。

## 当前环境验证

在 Windows x64 / Rust 1.96.0 执行：

本轮外框清理与静音图标调整重新执行 Windows 构建、Clippy、Rustfmt 和 UTF-8/diff 检查，未启动应用。下面的单元测试、脚本检查及早期界面运行记录沿用此前验证，本轮未重复执行。外框、静音图标、菜单、右键设置及不同缩放比例的实际表现交由用户验证。

- `cargo clippy --locked --offline --all-targets --target-dir target/gpui -- -D warnings`：通过。
- `cargo test --locked --offline --target-dir target/gpui`：38 项通过，1 项实际录屏测试按原配置跳过。新增测试覆盖 UTF-8/多行日志、并发完整性、子进程 panic 堆栈落盘、覆盖窗口的客户区定位，以及主面板和录制启动共用的目标类型校验。
- `cargo build --locked --offline`：通过，最终可执行文件为 `target/debug/ShiPing.exe`。
- Rustfmt、工作流 YAML、Bash/PowerShell 脚本语法、UTF-8 与 diff 空白检查：通过。
- Windows GPUI 主窗口在本机成功启动并显示中文。后续界面自动检查由用户按 Esc 停止；主题、设置页及完整交互由用户手动测试。
- 此前更新 `target/debug/ShiPing.exe` 时曾被旧程序占用；用户从托盘退出后，原路径构建成功并已更新。后续遇到程序占用时先请用户退出，再更新原路径。

macOS ARM64 交叉检查在依赖 `apple-cf 0.9.3` 的构建脚本阶段失败，提示找不到 Swift bridge 构建所需程序；当前 Windows 环境没有 Swift/Apple SDK，尚未完成应用类型检查或完整 macOS 构建。不能将源码适配和 CI 配置当作 macOS 编译通过。macOS x64 同样未在本机验证；需要 macOS runner 执行构建。

除上述用户确认的 Windows 副屏区域选择、以及用户提供的录制视频外，未验证修复后的真实录制、音频设备、输入穿透、失焦关闭、不同缩放比例、跨屏窗口移动、安装包安装或远程 CI；功能实测交由用户进行。旧版 Slint 的历史截图与录制测试不作为本次迁移通过的证据。

## 官方资料

- [GPUI Kit 0.7.3](https://docs.rs/gpui-kit/0.7.3/gpui_kit/)
- [GPUI Kit AllAssets](https://docs.rs/gpui-kit-assets/0.7.3/gpui_kit_assets/struct.AllAssets.html)
- [GPUI Kit IconName](https://docs.rs/gpui-kit-assets/0.7.3/gpui_kit_assets/enum.IconName.html)
- [GPUI Component Button](https://docs.rs/gpui-component/0.7.3/gpui_component/button/struct.Button.html)
- [GPUI Component TitleBar](https://docs.rs/gpui-component/0.7.3/gpui_component/struct.TitleBar.html)
- [GPUI Component Toggle](https://docs.rs/gpui-component/0.7.3/gpui_component/button/struct.Toggle.html)
- [GPUI Component PopupMenu](https://docs.rs/gpui-component/0.7.3/gpui_component/menu/struct.PopupMenu.html)
- [GPUI Component NativeMenu](https://docs.rs/gpui-component/0.7.3/gpui_component/native_menu/struct.NativeMenu.html)
- [GPUI Component Theme](https://docs.rs/gpui-component/0.7.3/gpui_component/struct.Theme.html)
- [GPUI Window](https://docs.rs/gpui-pre/0.3.9/gpui/struct.Window.html)
- [GPUI WindowOptions](https://docs.rs/gpui-pre/0.3.9/gpui/struct.WindowOptions.html)
- [GPUI Base Root](https://docs.rs/gpui-base/0.7.3/gpui_base/struct.Root.html)
- [GPUI window_shadow 示例](https://docs.rs/crate/gpui-pre/0.3.9/source/examples/window_shadow.rs)
- [GPUI App](https://docs.rs/gpui-pre/0.3.9/gpui/struct.App.html)
- [GPUI ForegroundExecutor](https://docs.rs/gpui-pre/0.3.9/gpui/struct.ForegroundExecutor.html)
- [GPUI Component Sidebar](https://docs.rs/gpui-component/0.7.3/gpui_component/sidebar/struct.Sidebar.html)
- [GPUI Component SidebarMenuItem](https://docs.rs/gpui-component/0.7.3/gpui_component/sidebar/struct.SidebarMenuItem.html)
- [tray-icon 0.21.2](https://docs.rs/tray-icon/0.21.2/tray_icon/)
- [Microsoft layered windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features#layered-windows)
- [Microsoft SetWindowPos](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)
- [Microsoft GetWindowRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowrect)
- [Microsoft GetClientRect](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect)
- [Microsoft ClientToScreen](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-clienttoscreen)
- [Apple ignoresMouseEvents](https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents)
- [log 0.4.33](https://docs.rs/log/0.4.33/log/)
- [Rust panic hook](https://doc.rust-lang.org/1.96.0/std/panic/fn.set_hook.html)
- [Rust Backtrace](https://doc.rust-lang.org/1.96.0/std/backtrace/struct.Backtrace.html)
