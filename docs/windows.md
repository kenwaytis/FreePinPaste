# Windows 支持

## 范围与发布

Windows x64 版共用 Linux 版的 GPUI 编辑器、标注、历史、PNG 导出及离线 OCR，不是第二套 GUI。目标为 **Windows 10 22H2 x64（Build 19045）**，运行需要可用的 Direct3D 11 图形驱动。

发行包：

```text
FreePinPaste-windows-x86_64.zip
FreePinPaste-0.1.0-windows-x86_64-setup.exe
FreePinPaste-windows-x86_64/
├── freepinpaste.exe
├── ocr/bin/tesseract.exe
├── ocr/tessdata/             # 现有 10 种语言，默认 chi_sim+eng
├── ocr/licenses/
├── licenses/                # Cargo 依赖清单及上游许可
└── docs/
```

- ZIP 必须完整解压，不可只移动 EXE。ZIP 是免安装包，配置仍在用户目录，不是无痕运行模式。
- 安装程序为当前用户安装，默认 `%LOCALAPPDATA%\Programs\FreePinPaste`，带开始菜单入口、可选桌面快捷方式和卸载。**不要求管理员权限，不默认启用自启动**。
- Rust 程序与 OCR 使用静态 CRT，不要求另装 VC++ Redistributable、Python 或系统 Tesseract。构建用的 Python 不属于运行依赖。
- 首版 EXE 和安装程序没有 Authenticode 签名，Windows 可能提示未知发布者。发布时附 SHA-256 校验文件；校验和不等同于代码签名。
- 配置：`%APPDATA%\freepinpaste\config.toml`。ZIP 和安装版共享配置；同一用户同一桌面会话内只运行一个实例。
- 用户在设置中打开自启动后，写入 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FreePinPaste`，值为带引号的当前 EXE 路径和 `--background`。移动 ZIP 目录后需重新设置。
- 卸载移除程序文件、菜单入口及指向本安装目录的自启动项，**保留用户配置**，不删除其他位置免安装版的启动项。

## 使用差异

Windows 不使用 Portal，不出现 GNOME 的授权截图、系统改键界面。默认 F1 截图、F3 贴图；可在设置中填写 `Ctrl+Shift+A` 等组合后保存。支持 F1–F24、字母、数字、Space、PrintScreen，以及 Ctrl / Shift / Alt / Win 修饰键。

热键冲突会显示错误，不能抢占别的程序。改键时先注册新绑定，成功后才保存配置；失败则保留原绑定及配置。开发机上的默认键已有冲突，因此自动化使用临时 Ctrl+Shift+F8 / F9 绑定，测试结束恢复原配置。

托盘菜单为截屏、截屏并自动复制、设置、重新启动、退出；左右键打开原生菜单。重启 Explorer 后重新注册托盘。无编辑器时贴图优先读取系统图片剪贴板，否则使用最近截图。图片复制使用 Windows 原生位图格式，可与原生应用互通；程序退出后图片仍保留在系统剪贴板。

保存使用原生 PNG 文件对话框，取消后恢复编辑器及标注，路径支持中文和空格。最终写入系统对话框确认的路径，不另行修改扩展名。

## 多屏、DPI 与边界

实现区分三套坐标：

1. 虚拟桌面物理坐标，可能有负原点；
2. 截图内部像素坐标，从图像左上角开始；
3. GPUI 窗口逻辑坐标，按窗口 DPI 缩放。

编辑器覆盖完整物理桌面，将截图映射到实际客户区；选区、标注和导出始终使用原图像素。工具栏、提示和贴图落点使用显示器物理边界换算，贴图原始尺寸按物理像素计算。截图窗口和贴图窗口绕过 GPUI 0.2.2 在 Windows 10 上保留的非客户区内缩，避免约 8 px 边框造成截图错位；贴图移动使用原生系统移动循环。

**已验收环境只有单屏 1920×1080 / 100%。** 多屏、负坐标与混合 DPI 的坐标实现和关键数学测试已加入，但本机没有第二块活动显示器，经用户确认暂不标记多屏实机验收通过。后续需覆盖：

- 主屏 100% + 副屏 150% / 200%；副屏放在主屏左侧、上方；
- 各屏截图及跨屏框选，导出尺寸和像素对齐；
- 贴图跨屏移动、缩放、原始尺寸恢复，工具栏不落入屏幕空隙。

截图为 **SDR** 路径，不提供 HDR 色彩管理。不捕获 UAC 安全桌面或绕过受保护内容限制；锁屏、远程会话断开、屏幕热插拔期间的行为未验收。

## 原生构建

需要 Rust MSVC 工具链（本机 1.96.0）、VS 2022 C++ Build Tools、Windows 10/11 SDK、CMake、Git、curl、tar，以及打包收集许可证所用的 Python。Release 构建需要 SDK 中的 `fxc.exe`；GPUI 支持通过 `GPUI_FXC_PATH` 指定路径。

在仓库目录运行：

```powershell
# 固定版本源码 + SHA-256 校验，全部工作文件位于 .cache/ 和 runtime/
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-ocr.ps1

cargo build --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --test ocr_smoke -- --ignored --nocapture

# 可选：准备用户范围的 Inno Setup 6.4.3 编译器
# 下载同时检查固定 SHA-256 与 Authenticode；不修改系统 PATH
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/prepare-windows-tools.ps1

# 生成 ZIP、安装程序和校验文件
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/package-windows.ps1
# 已构建 OCR 时可传 -SkipOcr；已有 Inno Setup 可用 -Iscc 指定 ISCC.exe
```

OCR 固定为 Tesseract 5.5.0、Leptonica 1.85.0 和现有 tessdata_fast 模型；静态链接 zlib、libpng、JPEG 和 TIFF。TIFF/JPEG 也用于 Leptonica 内置位图字体，不能因为输入是 PNG 就删除它们。Windows 构建关闭 OpenMP，不依赖 OpenMP DLL；识别仍在线程中运行，有 90 秒超时，子进程不弹控制台。

## 从 Linux 开发机联调

`windows-remote.py` 使用已有 SSH 别名，**不存储密码**。默认目录 `D:\project\FreePinPaste`，可通过参数更改：

```bash
python3 scripts/windows-remote.py sync
python3 scripts/windows-remote.py exec 'cargo test --locked; exit $LASTEXITCODE'
```

SSH 在 Session 0，不是用户交互桌面。不要在 SSH 中直接启动 GUI 并将成功退出等同于验收。`--ocr`、`--languages` 和构建测试可在 SSH 下运行；截图、贴图、设置和实例控制在相应用户桌面会话中运行。

已获用户允许的自动桌面验证：

```powershell
# 仅查询屏幕，不改变布局或缩放
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-windows-desktop.ps1

# 自动操作键鼠和剪贴板，需 Python + Pillow；先关闭已有 FreePinPaste
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-windows-desktop.ps1 -Arguments ""

# 改测安装包或解压包中的 EXE
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-windows-desktop.ps1 -Arguments '--exe "C:\path with spaces\freepinpaste.exe"'

# 在交互桌面验证热键冲突回滚与绑定交换（需要开发机 Rust）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-windows-desktop.ps1 -Arguments '--hotkey-test-only'

# 仅修改本应用的用户自启动值，测试结束恢复
cargo test --test windows_system -- --ignored --nocapture

# 自动安装、验证安装版和免安装版、卸载；拒绝覆盖已有的非测试安装
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-package.ps1
```

验证器创建当前用户的临时、非提权交互式计划任务，结束时删除任务。它会替换测试剪贴板、移动鼠标、输入按键、保存桌面截图，并临时修改本应用快捷键配置后恢复。不要在用户正在工作时运行。结果保存在 `.cache/windows-validation/`，不提交桌面截图或其他个人数据。

## 实现位置

- `src/platform/windows.rs`：Win32 物理桌面截图、窗口句柄、全尺寸客户区、置顶、移动与缩放。
- `src/desktop/windows.rs`：原生托盘消息循环、RegisterHotKey、快捷键原子切换、IFileSaveDialog。
- `src/clipboard/windows.rs`：arboard / Windows 原生图片格式读写。
- `src/service/windows.rs`：按 SID 和会话隔离的命名互斥体、命名管道；ACL 限定当前用户和 SYSTEM，并拒绝远程管道客户端。
- `src/config.rs`：用户配置及 HKCU 登录自启动。
- `packaging/windows/`：图标资源和用户级安装器；PerMonitorV2 manifest 由锁定版本 GPUI 提供，避免重复资源。

实际测试记录见 [validation-windows.md](validation-windows.md)。
