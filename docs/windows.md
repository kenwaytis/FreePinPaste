# Windows 支持

## 范围与发布

Windows x64 版共用 Linux 版的 GPUI 编辑器、标注、历史、PNG 导出及离线 OCR，不是第二套 GUI。支持 **Windows 10 22H2 x64（Build 19045）及 Windows 11 x64**，已实测 Windows 11 Build 26300；运行需要可用的 Direct3D 11 图形驱动。

发行包：

```text
FreePinPaste-windows-x86_64.zip
FreePinPaste-0.1.0-windows-x86_64-setup.exe
FreePinPaste-windows-x86_64/
├── freepinpaste.exe
├── ocr/bin/                  # lit.exe、pdfium.dll 与 app-local VC++ 运行库
├── ocr/models/ocria-nano/    # 模型、独立许可和 runtime/onnxruntime.dll
├── ocr/licenses/
├── licenses/                # Cargo 依赖清单及上游许可
└── docs/
```

- ZIP 必须完整解压，不可只移动 EXE。ZIP 是免安装包，配置仍在用户目录，不是无痕运行模式。
- 安装程序为当前用户安装，默认 `%LOCALAPPDATA%\Programs\FreePinPaste`，带开始菜单入口、可选桌面快捷方式和卸载。**不要求管理员权限，不默认启用自启动**。
- 主程序使用静态 CRT；原生 LiteParse 和 ONNX Runtime 使用包内的 x64 VC++ 运行库，无需 Python 或系统 OCR。
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

**已验收 Windows 10 单屏 1920×1080 / 100%，以及 Windows 11 双屏混合 DPI：**

- 主屏 3840×2160 / 175%，原点 `(0, 0)`；上方副屏 1920×1080 / 100%，原点 `(983, -1080)`。虚拟桌面为 3840×3240，存在负纵坐标和屏幕空隙。连接但未参与桌面的电视不计为活动屏，也未为测试启用。
- 各屏普通截图、反向快速框选、边缘选区、双向跨屏框选：导出尺寸准确，并与桌面参考图逐像素一致。
- 两屏边缘工具栏完整位于活动屏内，实际点击复制通过；贴图跨屏往返保持源像素尺寸、置顶及鼠标拖动锚点，滚轮缩放和双击恢复原始尺寸通过。
- 分数 DPI 输入坐标按源像素对齐；无边框图像使用 popup 窗口，避免系统最小尺寸限制；通过 `WM_GETDPISCALEDSIZE` 告知原生移动循环保持物理大小。

此结果不代表所有显示配置均已验证：副屏在左侧（负横坐标）、150% / 200% 组合、三块及更多活动屏仍需实机补验。

截图为 **SDR** 路径，不提供 HDR 色彩管理。不捕获 UAC 安全桌面或绕过受保护内容限制；锁屏、远程会话断开、屏幕热插拔期间的行为未验收。

## 原生构建

需要 Rust MSVC 工具链（本机 1.96.0）、VS 2022 C++ Build Tools、Windows 10/11 SDK、CMake、Git、curl、tar，以及打包收集许可证所用的 Python。Release 构建需要 SDK 中的 `fxc.exe`；GPUI 支持通过 `GPUI_FXC_PATH` 指定路径。

在仓库目录运行：

```powershell
# 固定版本原生发行包和权重 + SHA-256 校验，全部工作文件位于 .cache/ 和 runtime/
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

OCR 使用 LiteParse 2.21.0、OCRia-nano-1 v1.0.0 和 ONNX Runtime 1.31.1。构建脚本从 VS 的 `VCToolsRedistDir` 或 `vswhere` 找到 x64 VC++ 可再分发 DLL，随 `lit.exe` 放置。识别有 90 秒超时，子进程不弹控制台。模型的独立使用条款见发行包 `ocr/models/ocria-nano/LICENSE`。

## 从 Linux 开发机联调

`windows-remote.py` 使用已有 SSH 别名，**不存储密码**。默认目录 `D:\project\FreePinPaste`，可通过参数更改：

```bash
python3 scripts/windows-remote.py sync
python3 scripts/windows-remote.py exec 'cargo test --locked; exit $LASTEXITCODE'
```

SSH 在 Session 0，不是用户交互桌面。不要在 SSH 中直接启动 GUI 并将成功退出等同于验收。`--ocr`、`--ocr-info` 和构建测试可在 SSH 下运行；截图、贴图、设置和实例控制在相应用户桌面会话中运行。

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

多屏专项使用 `scripts/windows-multimon.py`，经同一入口在真实桌面运行；不改变显示布局或缩放：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run-windows-desktop.ps1 -Arguments '--multimon --exe "C:\path\freepinpaste.exe"'
# 可显式传入隔离的便携 Python，不要求全局安装或修改 PATH
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-package.ps1 -Python 'C:\test-tools\pythonw.exe' -MultiMonitor
```

专项脚本在各屏显示临时生成的测试图案，检查截图像素、工具栏完整性及贴图跨 DPI 移动；当前要求至少两块相邻活动屏。`-MultiMonitor` 使安装版与 ZIP 版各执行基础及多屏检查；结果分别保存为 `installed-multimon-result.json`、`portable-multimon-result.json`。没有覆盖的布局应记录为未验证，不能仅凭显示器枚举数量判定通过。

验证器创建当前用户的临时、非提权交互式计划任务，结束时删除任务。它会替换测试剪贴板、移动鼠标、输入按键、保存桌面截图，并临时修改本应用快捷键配置后恢复。不要在用户正在工作时运行。结果保存在 `.cache/windows-validation/`，不提交桌面截图或其他个人数据。

## 实现位置

- `src/platform/windows.rs`：Win32 物理桌面截图、窗口句柄、全尺寸客户区、置顶、移动与缩放。
- `src/desktop/windows.rs`：原生托盘消息循环、RegisterHotKey、快捷键原子切换、IFileSaveDialog。
- `src/clipboard/windows.rs`：arboard / Windows 原生图片格式读写。
- `src/service/windows.rs`：按 SID 和会话隔离的命名互斥体、命名管道；ACL 限定当前用户和 SYSTEM，并拒绝远程管道客户端。
- `src/config.rs`：用户配置及 HKCU 登录自启动。
- `packaging/windows/`：图标资源和用户级安装器；PerMonitorV2 manifest 由锁定版本 GPUI 提供，避免重复资源。

实际测试记录见 [validation-windows.md](validation-windows.md)。
