# FreePinPaste

面向 **Linux x86_64 / GNOME Wayland** 与 **Windows 10 22H2 / Windows 11 x64** 的截图、标注和贴图工具。GUI 使用 **Rust + GPUI**，不使用 Qt。Linux 通过桌面 Portal 截图，不依赖 GNOME 截图扩展或切换到 Xorg；Windows 使用原生桌面 API 与 DirectX 界面。

Windows 使用、构建和验收边界见 **[docs/windows.md](docs/windows.md)**。

这是第一版可运行实现，覆盖本次需求列出的工具，不宣称已经实现 Snipaste / 微信的全部功能。

## 已实现

- 默认 **F1 截图**、**F3 置顶**。Linux 通过 GlobalShortcuts Portal 注册并在系统界面改键；Windows 在应用设置中改键，注册成功后才保存，冲突时保留原有绑定。
- 完整桌面截图：Linux 使用 Screenshot Portal，包含原生 Wayland 窗口；Windows 使用原生 GDI 获取 SDR 桌面物理像素。
- 全桌面选区、8 个调整手柄、选区移动、像素尺寸显示；Linux 三屏横排 5760×1080 已联调。Windows 已通过 Windows 10 单屏及 Windows 11 双屏混合 DPI（175% / 100%）、负坐标和跨屏像素对齐验收。
- 浅色浮动主工具栏、蓝色选中态；矩形/椭圆、直线/箭头、画笔均有附属参数栏和两行色板。
- 文本输入、字号、粗体、斜体、字体配置。输入通过 GPUI Component 的文本控件处理。
- 像素橡皮擦：擦除**标注层**，不破坏原截图；擦除也能撤销。
- 撤销、还原（即**重做**）、退出、置顶、保存 PNG、复制 PNG。
- 独立无边框置顶贴图：拖动移动、滚轮缩放、Ctrl+C 复制；按设置双击或右键关闭，Esc 始终可关闭；右键关闭模式下双击恢复原始尺寸。
- 随包离线 CPU OCR：**OCRia-nano-1**，通过原生 LiteParse 和 ONNX Runtime 在本机识别，无需 GPU、Python 或首次使用时下载模型。
- 用户级开机自启动（实际为**登录图形桌面后启动**），默认关闭，在设置中启用。
- 单实例和本地命令转发；关闭设置或截图后后台继续运行。
- 系统托盘：截屏、截屏并自动复制、设置、重新启动、退出；快速模式框选松手即复制。
- 统一的「截图框＋图钉」应用 LOGO，内嵌于程序；Linux 提供 symbolic 托盘与用户图标目录安装，Windows 提供 EXE/窗口/托盘图标。

### 当前边界

- Linux 针对当前 GNOME Wayland + XWayland 环境；尚未验收混合 DPI、旋转屏幕或其他桌面环境。
- Windows 已验收 Windows 10 22H2 单屏和 Windows 11 Build 26300 双屏（4K 175% 主屏 + 上方 1080p 100% 副屏）；其他布局、旋转屏和热插拔未验收。不承诺 HDR 色彩管理、受保护视频、UAC 安全桌面或远程会话断开后的截图。
- Linux 托盘依赖 StatusNotifier/AppIndicator 宿主；没有宿主时仍可后台运行，可从启动器或 `freepinpaste --settings` 打开设置。Windows 使用系统通知区域，并在 Explorer 重启后重新注册图标。
- 暂无滚动长截图、窗口自动吸附、马赛克、贴图历史管理、标注对象二次选择编辑。这些不在本次首版工具清单内。
- 保存格式固定为 PNG；文件名严格使用系统保存对话框确认的路径，不偷偷修改扩展名。
- OCR 自动识别文字系统，返回可编辑的纯文本；面板不展示模型的边界框。

## Windows 发行包

提供 `FreePinPaste-windows-x86_64.zip` 和 `FreePinPaste-0.1.0-windows-x86_64-setup.exe`，均包含 OCRia-nano-1 离线运行时。

- **ZIP**：完整解压后运行 `freepinpaste.exe`，不要只复制 EXE。
- **安装程序**：当前用户安装，默认目录 `%LOCALAPPDATA%\Programs\FreePinPaste`，创建开始菜单入口并提供卸载；无需管理员权限。
- 不需要另外安装 Rust、Python 或系统 OCR；识别所需的 VC++ 运行库放在包内。首版安装程序未签名，Windows 可能提示未知发布者。
- Windows 不需要「授权截图」。F1 / F3 被其他程序占用时，在设置中改为其他组合，例如 `Ctrl+Shift+A`，保存后生效。
- ZIP 与安装版共享 `%APPDATA%\freepinpaste\config.toml`。自启动默认关闭；移动 ZIP 目录后应重新设置自启动。更新前请先从托盘退出，卸载保留用户配置。

详细开发与验证方法见 [Windows 文档](docs/windows.md)。

## Linux 发行包

构建产物：

```text
dist/FreePinPaste-linux-x86_64/
├── freepinpaste
├── ocr/                    # 原生 LiteParse、PDFium、OCRia 模型、ONNX Runtime 及许可
├── licenses/
├── install.sh
└── README.md
```

直接运行：

```bash
./dist/FreePinPaste-linux-x86_64/freepinpaste --settings
```

安装到当前用户（无需 root）：

```bash
# 安装/更新前关闭已有实例
./dist/FreePinPaste-linux-x86_64/freepinpaste --quit
./dist/FreePinPaste-linux-x86_64/install.sh
~/.local/bin/freepinpaste --settings
```

安装位置为 `${XDG_DATA_HOME:-$HOME/.local/share}/freepinpaste/app`，入口为 `~/.local/bin/freepinpaste`。包面向当前系统，不是任意发行版都能运行的 AppImage；GPUI 仍使用系统 Vulkan 驱动、X11、字体和 glibc。

### Linux 首次授权非常重要

1. 启动后，在 GNOME 弹窗中确认全局快捷键。F1 / F3 是首选值，最终以系统确认结果为准。
2. 在**前台设置窗口**点击右上角 **「授权截图」**，允许系统截图权限。
3. 授权成功后，程序丢弃含设置窗口的授权截图，隐藏设置，再获取一张干净的桌面图像进入编辑器。
4. 以后 F1 直接截图。已有授权从 Portal PermissionStore **只读查询**，程序不修改或伪造权限。

GNOME 51 不允许后台应用弹出首次截图授权对话框。不能一开始就隐藏窗口，否则 Portal 返回 `Other`，系统日志为 `Only the focused app is allowed to show a system access dialog`。本项目已为此实现专门的授权流程。

### 托盘

托盘以菜单操作为主，不绑定单击截屏；实际鼠标手势由桌面宿主决定。菜单按以下顺序排列：

| 菜单 | 行为 |
|---|---|
| 截屏 | 普通框选、标注流程，与 F1 相同 |
| 截屏并自动复制 | 拖动框选，松开鼠标后立即复制选区并关闭；不显示标注工具栏；Esc / 右键取消；小于 2×2 像素的选区不复制 |
| 设置 | 打开或激活现有设置窗口 |
| 重新启动 | 不弹确认；关闭截图与所有贴图后后台重启，不打开设置；保留已保存的配置，不恢复未保存的编辑、贴图或最近截图 |
| 退出 | 结束整个程序，包括所有窗口和托盘 |

Linux 首次截屏需在设置窗口中授权；从自动复制入口发起时，授权后继续自动复制模式。Windows 不需要这个授权步骤。截图请求处理中不会重复发起；已有同模式编辑窗口时激活该窗口，切换普通 / 自动复制模式则关闭当前编辑并重新截屏。

Linux 复制后的图片由后台进程提供；重启或退出后的保留行为取决于桌面剪贴板管理器。Windows 图片交给系统剪贴板持有，退出程序后仍可粘贴。重新启动使用当前可执行文件，不是自动更新或重新构建程序。

### 操作

| 操作 | 按键 / 方式 |
|---|---|
| 截图 | F1（Linux 由系统改键，Windows 在应用设置中改键） |
| 置顶 | F3：编辑中置顶选区；其他时候优先读取剪贴板图片，否则使用最近截图 |
| 框选 | 按住左键拖动；拖手柄调整大小 |
| 移动选区 | 空格切到选区工具，再在选区内拖动 |
| 矩形 / 椭圆 | R / 第一项工具，下方切换形状、线宽、填充、颜色 |
| 直线 / 箭头 | L / 第二项工具，下方切换类型、线宽、颜色 |
| 自由画笔 | P |
| 文本 | T，点击选区确定文字位置；输入后确认或 Ctrl+Enter |
| 橡皮擦 | E；参数栏显示实际擦除宽度 |
| OCR | O；在结果框编辑或复制识别文本 |
| 约束绘制 | Shift：正方形/圆，或 45° 增量直线/箭头 |
| 撤销 / 重做 | Ctrl+Z / Ctrl+Shift+Z 或 Ctrl+Y |
| 保存 | Ctrl+S / 保存按钮 |
| 复制 | Enter / Ctrl+C / 复制按钮；选区模式双击也可复制 |
| 退出截屏 | Esc / ×；右键先返回选区工具，再右键退出 |
| 关闭贴图 | 默认双击左键；可在设置的「贴图 → 关闭方式」改为右键关闭；Esc 始终可关闭 |
| 移动 / 缩放贴图 | 左键拖动 / 滚轮；选择右键关闭时，双击左键恢复原始尺寸 |

Linux 图片复制由独立 X11 selection owner 提供 `image/png`，应用在后台存活时持续服务剪贴板。已经验证通过 Mutter 桥接到原生 Wayland 应用。不能直接依赖 GPUI 0.2.2 的 `write_to_clipboard(Image)`：该版本 X11 实现实际只写文本。

### 离线 OCR：OCRia-nano-1

编辑器和命令行均使用 LiteParse 的页面解析流程，以 **250 DPI** 渲染截图，优先照顾小字。模型逐行自动选择拉丁、中日韩、RTL 或其他文字识别器，无需设置语言；旧配置中的 `ocr_languages` 将被忽略。模型缺失时直接报错，不回退其他引擎。

桌面程序启动后在后台预加载模型，所有窗口串行复用一个本地 OCR 子进程，不监听网络端口。隐藏到托盘时保持预热；退出或重启时终止子进程。识别超时（90 秒）会终止卡住的子进程并清理临时文件，下次识别重新加载。`--ocr FILE` 始终独立运行随包 `lit`，不启动或连接托盘。

**模型使用条款：**使用或再分发随包 OCR 权重及配套数据必须遵守 [LlamaIndex 的 OCRia-nano-1 模型许可](https://huggingface.co/llamaindex/OCRia-nano-1/blob/main/LICENSE)，包括第 5 条及附件 A 的用途限制；该部分不适用 FreePinPaste 的 Apache-2.0。上年总收入或累计融资超过 200 万美元的组织有个人/研究以外的用途限制；竞争产品或服务另受限制，与规模无关。完整条款随包位于 `ocr/models/ocria-nano/LICENSE`。

```bash
freepinpaste --ocr-info
freepinpaste --ocr /path/to/image.png
```

## Linux 源码构建

本机已使用 Rust 1.96.0、CMake、GCC 和系统开发库成功构建。Cargo 锁文件纳入仓库。依赖包括 X11/XKB、Wayland 开发库（GPUI Component 传递启用）、fontconfig/freetype、Vulkan 等；准备 OCR 运行时需要 Python 3、curl 和 Rust 构建工具链（同时编译 `ocr-worker/`）。

```bash
# 准备固定版本的 OCR 运行时，并编译常驻子进程到 target/ocr-worker
./scripts/build-ocr.sh

cargo build --locked
cargo run --locked -- --settings

# 生成带内置 OCR 的 tar.gz，耗时取决于首次 Rust 编译
./scripts/package.sh
```

LiteParse CLI、模型和 ONNX Runtime 的下载内容均校验 SHA-256。当前 Linux x64 的完整 OCR 目录约 89 MiB。`runtime/`、`target/`、`dist/` 和 `.cache/` 不提交到 Git。

如果 Cargo 网络遇到 TLS 错误，可尝试 `CARGO_HTTP_MULTIPLEXING=false cargo build --locked`；依赖已缓存时可使用 `--offline`。

### 开发命令

```bash
cargo run -- --capture
cargo run -- --capture-copy
cargo run -- --restart
cargo run -- --pin
cargo run -- --settings
cargo run -- --background
cargo run -- --quit
cargo run -- --edit /absolute/path/to/image.png
# 不弹全局快捷键授权，供本地 UI 调试；已有实例会接收命令
cargo run -- --no-shortcuts --edit /absolute/path/to/image.png
```

`--edit` 会把本地图片放入全屏编辑器；用于排查编辑器问题，与真实 Portal 截图无关。`--install-desktop` 可显式把桌面入口更新为当前可执行文件路径。

贴图关闭方式保存后立即对已打开和新建贴图生效。配置项 `pin_close_action` 可选 `"double_click"`（默认）或 `"right_click"`；旧配置未包含该项时也默认双击关闭。

Linux 配置文件：`~/.config/freepinpaste/config.toml`。开机自启动入口：`~/.config/autostart/io.github.freepinpaste.FreePinPaste.desktop`。也遵循 `XDG_CONFIG_HOME` / `XDG_DATA_HOME`。

已有快捷键的改动请点 **「系统改键」**，调用 GlobalShortcuts v2 `ConfigureShortcuts`。设置中的“默认截图 / 默认置顶”用于首次绑定的首选值，不能覆盖用户已经在系统中批准的绑定。Portal 服务重启后需重启本程序重新建立快捷键 session。

## 测试

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
# 需要先构建内置 OCR 和本机中文字体
cargo test --test ocr_smoke -- --ignored --nocapture

# 可选剪贴板互通测试：会替换当前剪贴板，持续提供 PNG 20 秒
cargo run --example clipboard_probe -- /path/to/image.png
# 在另一个终端执行或直接粘贴到 Wayland 应用
/usr/bin/wl-paste --type image/png > /tmp/copied.png
```

自动化覆盖选区反向拖动与裁剪、工具栏边界、历史分支、橡皮擦与撤销、PNG 像素往返、配置、启动项创建/移除及旧配置迁移。另有真实 CPU 中英混排 OCR 测试。

当前 GNOME 对 XTest 的模拟鼠标事件不生效，因此不把自动点击脚本当作 GUI 验收依据。实际授权、F1、框选、绘图、保存和复制已由用户现场操作确认；置顶已检查 `_NET_WM_STATE_ABOVE`。详细记录见 [docs/validation.md](docs/validation.md)。

## 结构

```text
src/
├── main.rs, service/         # 启动、后台重启、Unix socket / Windows 本地命名管道
├── tray_linux.rs, branding.rs # StatusNotifier 托盘、内嵌 LOGO 和图标安装
├── desktop/                 # Portal / Windows 截图、快捷键、文件对话框、Windows 托盘
├── platform/                # X11 / Win32 窗口、显示器几何、置顶和 DPI
├── clipboard/               # X11 image/png / Windows 原生图片剪贴板
├── document.rs, raster.rs    # 像素坐标文档、历史、统一标注渲染和 PNG 导出
├── ocr.rs, config.rs         # 随包 OCR、配置和自启动
└── ui/                      # GPUI 设置、编辑器、附属工具栏、贴图
```

选区和标注统一保存在**截图原始像素坐标**。UI 坐标在输入时转换；导出、复制、OCR、贴图共用同一份渲染结果，不各自实现一套绘制逻辑。Linux 截图不使用 X11 读取桌面；X11 只管理本程序窗口与剪贴板。Windows 坐标保留虚拟桌面的负原点，截图编辑器使用物理像素统一映射；原始尺寸贴图在不同 DPI 下保持源像素尺寸。

GPUI 0.2.2 的兼容处理：X11 `HasWindowHandle` 未实现，因此使用本进程 PID + 临时唯一标题定位自己创建的窗口；后台 keeper 通过 WM 的隐藏状态保活，不把 1×1 窗口留在桌面；`xattr 0.2.3` 的旧 Linux errno 别名在 `vendor/xattr` 做了一个兼容补丁，详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 协议与上游

- [Screenshot Portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Screenshot.html)
- [GlobalShortcuts Portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)
- [Host Registry](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html)
- [EWMH 窗口状态和全屏多屏协议](https://specifications.freedesktop.org/wm-spec/latest/)
- [ksni / StatusNotifier 托盘](https://docs.rs/ksni/0.3.7/ksni/)
- [GPUI 0.2.2](https://docs.rs/gpui/0.2.2/gpui/)
- [LiteParse](https://github.com/run-llama/liteparse) / [OCRia-nano-1](https://huggingface.co/llamaindex/OCRia-nano-1)
