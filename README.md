# FreePinPaste

面向当前 **Linux x86_64 / GNOME Wayland** 桌面的截图、标注和贴图工具。GUI 使用 **Rust + GPUI**；不需要 GNOME 扩展，不需要切换到 Xorg，也不使用 Qt。

这是第一版可运行实现，覆盖本次需求列出的工具，不宣称已经实现 Snipaste / 微信的全部功能。

## 已实现

- 默认 **F1 截图**、**F3 置顶**，通过 GlobalShortcuts Portal 注册；支持在系统授权界面改键，设置页显示实际绑定结果。
- Screenshot Portal 获取完整桌面，包括原生 Wayland 窗口。
- 全桌面选区、8 个调整手柄、选区移动、像素尺寸显示；当前三屏横排 5760×1080 已联调。
- 浅色浮动主工具栏、蓝色选中态；矩形/椭圆、直线/箭头、画笔均有附属参数栏和两行色板。
- 文本输入、字号、粗体、斜体、字体配置。输入通过 GPUI Component 的文本控件处理。
- 像素橡皮擦：擦除**标注层**，不破坏原截图；擦除也能撤销。
- 撤销、还原（即**重做**）、退出、置顶、保存 PNG、复制 PNG。
- 独立无边框置顶贴图：拖动移动、滚轮缩放、双击原始尺寸、右键或 Esc 关闭、Ctrl+C 复制。
- 随包离线 CPU OCR：**Tesseract 5.5.0 + Leptonica 1.85.0 + tessdata_fast**，不调用云端，不需要 Python 或系统 Tesseract。
- 用户级开机自启动（实际为**登录图形桌面后启动**），默认关闭，在设置中启用。
- 单实例和本地命令转发；关闭设置或截图后后台继续运行。

### 当前边界

- 只针对当前 GNOME Wayland + XWayland 环境；尚未验收混合 DPI、旋转屏幕或其他桌面环境。
- 暂无托盘图标；可从应用启动器重新打开 FreePinPaste 设置，或运行 `freepinpaste --settings`。
- 暂无滚动长截图、窗口自动吸附、马赛克、贴图历史管理、标注对象二次选择编辑。这些不在本次首版工具清单内。
- 保存格式固定为 PNG；文件名严格使用系统保存对话框确认的路径，不偷偷修改扩展名。
- OCR 的 10 种语言可组合选择，不等于自动语言检测。低清晰度、手写、复杂排版不在首版识别质量承诺内。

## 使用发行包

构建产物：

```text
dist/FreePinPaste-linux-x86_64/
├── freepinpaste
├── ocr/                    # 引擎、模型、所需非 glibc 动态库及许可
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

### 首次授权非常重要

1. 启动后，在 GNOME 弹窗中确认全局快捷键。F1 / F3 是首选值，最终以系统确认结果为准。
2. 在**前台设置窗口**点击右上角 **「授权截图」**，允许系统截图权限。
3. 授权成功后，程序丢弃含设置窗口的授权截图，隐藏设置，再获取一张干净的桌面图像进入编辑器。
4. 以后 F1 直接截图。已有授权从 Portal PermissionStore **只读查询**，程序不修改或伪造权限。

GNOME 51 不允许后台应用弹出首次截图授权对话框。不能一开始就隐藏窗口，否则 Portal 返回 `Other`，系统日志为 `Only the focused app is allowed to show a system access dialog`。本项目已为此实现专门的授权流程。

### 操作

| 操作 | 按键 / 方式 |
|---|---|
| 截图 | F1（可由系统改键） |
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

图片复制由独立 X11 selection owner 提供 `image/png`，应用在后台存活时持续服务剪贴板。已经验证通过 Mutter 桥接到原生 Wayland 应用。不能直接依赖 GPUI 0.2.2 的 `write_to_clipboard(Image)`：该版本 X11 实现实际只写文本。

### OCR 语言

| 模型名 | 语言 |
|---|---|
| `chi_sim` / `chi_tra` | 简体中文 / 繁体中文 |
| `eng` | 英语 |
| `jpn` / `kor` | 日语 / 韩语 |
| `fra` / `deu` / `spa` | 法语 / 德语 / 西班牙语 |
| `rus` / `ara` | 俄语 / 阿拉伯语 |

默认 `chi_sim+eng`。在设置中用 `+` 组合，例如 `jpn+eng`。只加载所选语言，不把全部模型同时塞进 CPU。OCR 在工作线程运行，限制 OpenMP 线程数为 4，单次识别超时 90 秒；输入和结果临时文件在任务结束后删除。

```bash
freepinpaste --languages
freepinpaste --ocr /path/to/image.png
```

## 从源码构建

本机已使用 Rust 1.96.0、CMake、GCC 和系统开发库成功构建。Cargo 锁文件纳入仓库。依赖包括 X11/XKB、Wayland 开发库（GPUI Component 传递启用）、fontconfig/freetype、Vulkan、libpng/zlib/libtiff；不需要系统安装 Tesseract。

```bash
# 仅首次需要下载 OCR 源码和语言数据；安装仅写项目 runtime/ 与 .cache/
./scripts/build-ocr.sh

cargo build --locked
cargo run --locked -- --settings

# 生成带内置 OCR 的 tar.gz，耗时取决于首次 Rust 编译
./scripts/package.sh
```

源代码和模型固定到提交，下载内容校验 SHA-256。OCR 数据约 24 MB，完整 OCR 目录约 28 MB（最终发行包另含依赖库）。`runtime/`、`target/`、`dist/` 和 `.cache/` 不提交到 Git。

如果 Cargo 网络遇到 TLS 错误，可尝试 `CARGO_HTTP_MULTIPLEXING=false cargo build --locked`；依赖已缓存时可使用 `--offline`。

### 开发命令

```bash
cargo run -- --capture
cargo run -- --pin
cargo run -- --settings
cargo run -- --background
cargo run -- --quit
cargo run -- --edit /absolute/path/to/image.png
# 不弹全局快捷键授权，供本地 UI 调试；已有实例会接收命令
cargo run -- --no-shortcuts --edit /absolute/path/to/image.png
```

`--edit` 会把本地图片放入全屏编辑器；用于排查编辑器问题，与真实 Portal 截图无关。`--install-desktop` 可显式把桌面入口更新为当前可执行文件路径。

配置文件：`~/.config/freepinpaste/config.toml`。开机自启动入口：`~/.config/autostart/io.github.freepinpaste.FreePinPaste.desktop`。也遵循 `XDG_CONFIG_HOME` / `XDG_DATA_HOME`。

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

自动化覆盖选区反向拖动与裁剪、工具栏边界、历史分支、橡皮擦与撤销、PNG 像素往返、配置、启动项创建/移除及语言校验。另有真实 CPU 中英混排 OCR 测试。

当前 GNOME 对 XTest 的模拟鼠标事件不生效，因此不把自动点击脚本当作 GUI 验收依据。实际授权、F1、框选、绘图、保存和复制已由用户现场操作确认；置顶已检查 `_NET_WM_STATE_ABOVE`。详细记录见 [docs/validation.md](docs/validation.md)。

## 结构

```text
src/
├── main.rs, service.rs       # 启动、单实例 Unix socket
├── portal.rs                # 应用身份、首次授权、截图、快捷键、文件对话框
├── platform.rs              # 本进程 X11 窗口、三屏全屏和置顶
├── clipboard.rs             # 真实 image/png selection owner
├── document.rs, raster.rs    # 像素坐标文档、历史、统一标注渲染和 PNG 导出
├── ocr.rs, config.rs         # 随包 OCR、配置和自启动
└── ui/                      # GPUI 设置、编辑器、附属工具栏、贴图
```

选区和标注统一保存在**截图原始像素坐标**。UI 坐标在输入时转换；导出、复制、OCR、贴图共用同一份渲染结果，不各自实现一套绘制逻辑。截图不使用 X11 读取桌面；X11 只管理本程序窗口与剪贴板。

GPUI 0.2.2 的兼容处理：X11 `HasWindowHandle` 未实现，因此使用本进程 PID + 临时唯一标题定位自己创建的窗口；后台 keeper 通过 WM 的隐藏状态保活，不把 1×1 窗口留在桌面；`xattr 0.2.3` 的旧 Linux errno 别名在 `vendor/xattr` 做了一个兼容补丁，详见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 协议与上游

- [Screenshot Portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Screenshot.html)
- [GlobalShortcuts Portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html)
- [Host Registry](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html)
- [EWMH 窗口状态和全屏多屏协议](https://specifications.freedesktop.org/wm-spec/latest/)
- [GPUI 0.2.2](https://docs.rs/gpui/0.2.2/gpui/)
- [Tesseract](https://github.com/tesseract-ocr/tesseract) / [tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast)
