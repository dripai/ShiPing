# ShiPing 独立项目迁移

## GPUI 迁移（2026-10-14）

- 当前项目为单个 Cargo package，源码集中在 `src/`；UI 使用 GPUI Kit 0.7.3，Slint 依赖、构建步骤和视图已移除。
- 本次适配目标为 Windows 和 macOS；Linux 源码、依赖和打包材料保留，已从当前 CI/发布构建矩阵排除。
- 新增跟随系统、浅色、深色主题；设置页分为常规、录制和快捷键。主界面布局暂时保留，后续重新设计。
- 当前验证及平台适配说明见 [GPUI 迁移说明](docs/GPUI-MIGRATION.md)。以下记录属于此前的目录整理和独立仓库拆分。

## 目录整理历史（2026-10-13，GPUI 迁移前）

- 根目录 `Cargo.toml` 管理单个 `shiping` package，保留版本 `0.1.34` 和 `ShiPing` 可执行文件名。
- Rust 源码集中到 `src/`，当时的 Slint 视图与复用组件位于 `src/ui/views` 和 `src/ui/components`。
- 国际化并入 `src/i18n.rs`，配置目录函数并入 `src/config.rs`；删除未使用的共享日志和图片目录辅助代码。
- 当时的图标与翻译位于 `assets/` 和 `translations/`；打包材料统一到 `packaging/windows`、`packaging/macos`、`packaging/linux`。
- 原 `apps/`、`crates/` 目录已移除，配置路径、录制逻辑及平台包身份保持不变。
- 本次整理已通过 Windows release 编译、29 项测试（另 1 项真实录屏测试按原配置跳过）、Clippy、Rustfmt、脚本语法、工作流 YAML、UTF-8 和完整 diff 检查；`./start.sh build` 与本地 MSIX/MSIXUPLOAD 打包通过，包内程序、图标和应用身份已核对。
- 601 个外部依赖的版本、解析后的功能开关和锁文件校验和保持不变。未重新执行实机录屏、完整 UI 交互、macOS/Linux 构建或安装测试，也未运行远程 CI。
- 以下内容为 2026-10-01 的历史记录，其中的目录名和验证结果不代表本次整理后的验证状态。

## 独立仓库迁移历史（2026-10-01）

迁移日期：2026-10-01。版本保持 `0.1.34`，Slint 保持 `1.17.0`。

- 来源：`D:/workspace/12.aiwork/ShiTu`，提交 `4160e0ea1e56161316df102d748a82be81b4ba65`。
- 目标：`D:/workspace/12.aiwork/ShiPing`，远程 `git@github.com:dripai/ShiPing.git`。
- 原提交及祖先历史保留在 Git 中，可通过 `git log -- apps/shiping` 追溯；原发布标签不导入新项目。
- `apps/shiping`、两个内部共享 crate、翻译、图标、平台打包材料和截图迁入新仓库。
- 工作区、默认构建入口、锁文件、CI、版本标签发布、README、隐私政策和反馈链接独立维护。
- `shi-foundation`、`shi-ui` 在两个项目中分别维护，不依赖兄弟目录或跨仓库链接。
- 两个仓库分别通过 `.cargo/config.toml` 使用本地 `target/`；避免全局构建目录中缓存的绝对路径串到另一个项目。
- Windows 配置仍使用 `%APPDATA%/ShiPing/config.json`。各平台配置和录制输出实现保持不变。
- Store 清单、发布者和包身份，以及 macOS bundle ID、Linux desktop ID 保持不变。
- UI 只改已有 GitHub 链接的文字和地址，不新增组件、窗口或交互。
- 本次本地拆分不执行 Git 提交、推送、版本标签发布或 Store 上传。

## 验证

- ShiPing：`cargo check --release --locked --package shiping`、`cargo test --release --locked --workspace` 通过（28 项录屏测试和 1 项基础模块测试）。
- ShiPing：单独执行原有 `records_a_short_mp4` 硬件测试通过，在当前 Windows 设备实际采集并生成短时 MP4；这不覆盖系统声音、麦克风、长时录制或完整 UI 交互。
- ShiPing：`cargo clippy --release --locked --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、独立 `./start.sh build` 通过。
- ShiPing：MakeAppx 成功生成 `.msix` 和 `.msixupload`；核对包身份、`0.1.34.0` 版本、包内可执行文件 SHA-256 和上传归档内容。
- ShiTu：`cargo check --release --locked --package shitu`、`cargo test --release --locked --workspace`（26 项截图测试和 1 项基础模块测试）、`cargo fmt --all -- --check` 和独立 `./start.sh build` 通过。
- ShiTu：MakeAppx 成功生成 `.msix` 和 `.msixupload`；核对原包身份、`0.1.34.0` 版本、包内可执行文件 SHA-256 和上传归档内容。
- 两个工作区：依赖版本及校验和保持原锁文件中的值；工作流 YAML、脚本语法、默认产品、独立构建目录、产品参数拒绝和 UTF-8 检查通过。
- 56 个录屏实现、共享模块、资源和打包身份文件与迁移前逐字节一致；3 个录屏文件仅调整应用描述、发布说明和 GitHub 链接。
- ShiTu 应用及共享模块无代码改动。移除的原始文件及本地设计文档保留在原仓库忽略目录 `.codex-tmp/shiping-split-source-backup/`；常规 Git diff 不包含该备份。
- macOS/Linux 构建及真机录制、GitHub Actions 运行、Store 安装和提交审核尚未在本次迁移中验证。没有把已有 CI 配置当作本次运行结果。

## 官方依据

- [Cargo 1.96 workspace](https://doc.rust-lang.org/1.96.0/cargo/reference/workspaces.html)
- [Cargo 1.96 配置与构建目录](https://doc.rust-lang.org/1.96.0/cargo/reference/config.html#buildtarget-dir)
- [Git fetch](https://git-scm.com/docs/git-fetch)
- [Git read-tree](https://git-scm.com/docs/git-read-tree)
- [Slint 1.17 原生组件](https://github.com/slint-ui/slint/blob/v1.17.0/internal/compiler/widgets/fluent/std-widgets.slint)
- [GitHub Actions 任务依赖](https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/using-jobs-in-a-workflow)
- [MakeAppx](https://learn.microsoft.com/en-us/windows/msix/package/create-app-package-with-makeappx-tool)
- [Microsoft Store 上传包](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/upload-app-packages)
