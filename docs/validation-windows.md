# Windows 验证记录

## Windows 10 基线环境

- Windows 10 专业版 x64，22H2 / Build 19045；本地交互式桌面 Session 1。
- Rust 1.96.0 MSVC，VS 2022 Build Tools 17.14，MSVC 14.44，Windows SDK 10.0.26100.0。
- NVIDIA GeForce RTX 5060 Ti；当前活动屏幕为 1920×1080，96 DPI / 100%。
- 通过 SSH 在 Session 0 构建；桌面测试使用当前用户、非提权、临时交互式计划任务。

## 自动化检查

- Linux：`cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、17 项单元测试、真实中英文 CPU OCR 测试通过。
- Windows：原生编译、Clippy、15 项常规单元测试通过；真实 CPU OCR 和自启动注册/移除测试通过。
- 热键冲突回滚、绑定交换测试在交互式桌面单独运行通过；RegisterHotKey 不能在 SSH 的非交互窗口工作站测试，因此默认测试集将其标记为 ignored。
- Windows Release 原生构建通过；使用 SDK 的 fxc 编译 GPUI shaders。
- `dumpbin /DEPENDENTS`：主程序只导入 Windows 系统 DLL，Tesseract 只导入 KERNEL32.dll；不依赖 VC++ Redistributable 或额外 OCR DLL。
- OCR 10 种语言模型通过 `packaging/models.sha256` 校验。

## 已完成的桌面检查

桌面自动化入口：`scripts/run-windows-desktop.ps1` + `scripts/windows-smoke.py`。

- 设置窗口正常创建，关闭后后台保活；重复启动转发至已有实例。
- 全屏截图编辑器外框及客户区覆盖 1920×1080，截图选区对应真实物理像素。
- 实际全局快捷键触发截图与贴图；默认键冲突明确报错，不抢占其他程序。
- 420×220 选区在 `(200,180)` 创建同尺寸置顶贴图；原生窗口置顶标志有效。
- Ctrl+C 输出系统图片剪贴板，原生读取尺寸为 420×220。
- 贴图拖动、滚轮缩放（420×220 → 462×242）及 Esc 关闭。
- Windows Forms 写入的外部 80×60 位图可直接贴图，默认双击关闭有效。
- PNG 原生保存对话框；取消后恢复编辑器，再保存至包含中文的路径，输出尺寸正确。
- 文本面板自动聚焦、粘贴中英文、Ctrl+Enter 确认；矩形标注、Ctrl+Z 撤销、Ctrl+Y 重做，像素对比通过。导出图片经真实离线 OCR 得到 `FreePinPaste 中文 2026`。
- 原生托盘五项菜单渲染及点击「设置」通过。
- 快速模式框选松手复制；后台重启产生新进程，保留用户配置；退出后系统图片剪贴板仍保留。

测试临时使用 Ctrl+Shift+F8 / F9，不修改产品默认 F1 / F3；测试配置与自启动值在结束后恢复。验证截图、日志、OCR 输出位于开发机 `.cache/windows-validation/`，不纳入 Git。

## 发布包验证

用户级安装程序已安装至独立测试目录，创建当前用户卸载项、开始菜单快捷方式和完整 OCR 目录。

`scripts/test-windows-package.ps1` 已实际通过：

- 安装版完成上述 **15 项桌面检查**，包括使用随安装目录附带的 OCR。
- ZIP 解压到含空格路径后，同样完成 **15 项桌面检查**，不依赖开发环境 OCR 覆盖变量。
- 用户级卸载成功，程序 EXE、卸载注册项和开始菜单入口均已移除。
- 临时计划任务及测试进程清理完成；本应用用户配置恢复到测试前状态，自启动测试值已恢复。

结果保存在 `.cache/windows-validation/package-result.json`、`installed-result.json` 和 `portable-result.json`。

## 联调中修复的问题

- GPUI Windows 10 无标题栏窗口的非客户区内缩：避免 420×220 框选被错误换算为约 424×222。
- GPUI Windows 后端没有实现通用窗口移动入口：改用原生系统移动循环，并校验实际位移和缩放。
- 文本面板初始焦点被画布自动聚焦覆盖，以及 Ctrl+Enter 被子输入控件消费：分别抑制画布默认聚焦、在 action capture 阶段确认文本。
- Cargo 部分许可证带有 1970 年时间戳：仅对生成的发行目录修正 ZIP 不支持的时间戳，不改动依赖缓存。

## Windows 11 双屏混合 DPI 补验

### 环境与执行边界

- Windows 11 专业版 x64，Build 26300，NVIDIA GeForce RTX 4060，驱动 32.0.16.2114。
- 真实控制台桌面 Session 1，`EnumDisplayMonitors` 枚举 **两块**活动屏。WMI 另报告连接的电视，不能据此当作三屏桌面；未启用电视。
- 主屏 `(0, 0)–(3840, 2160)`，168 DPI / 175%；副屏 `(983, -1080)–(2903, 0)`，96 DPI / 100%。虚拟桌面 `(0, -1080)–(3840, 2160)`。
- 通过现有 FRP STCP SSH 访客隧道传输文件；GUI 测试通过当前用户、非提权交互式临时计划任务运行，不在 SSH Session 0 冒充桌面测试。
- 用户批准自动键鼠、剪贴板和截图验证，未改变布局、缩放或电视启用状态；未重启 Explorer、注销或重启电脑。
- 验证工具使用隔离的 Python 3.12.10 embeddable + Pillow 11.3.0，不修改系统 PATH；Python EXE 的 Python Software Foundation 签名有效，Pillow 下载校验 PyPI 提供的 SHA-256。应用及 OCR 不依赖这些测试工具。

### 结果

- `windows-smoke.py` 的 **15 项基础桌面检查通过**：设置、单实例、物理桌面覆盖、实际全局热键、贴图尺寸与置顶、剪贴板、移动缩放、外部位图、中文路径保存及取消、标注撤销重做、OCR、快速截图、托盘菜单、后台重启及退出后剪贴板保留。OCR 得到 `FreePinPaste 中文 2026`。
- `windows-multimon.py` 的 **14 项多屏检查通过**：
  - 两块屏分别普通框选 420×220、反向快速框选 173×97、边缘框选 220×140；上下屏交界双向框选 420×280。共 8 项均与截图前桌面参考图逐像素一致。
  - 普通及跨屏框选检查工具栏色块边界、完整宽高及其落在活动屏内；两屏边缘实际点击最右端复制按钮成功。
  - 在两块屏分别创建 80×60 贴图，6 次移动/缩放/恢复检查覆盖 175% → 100% → 175%：窗口保持 80×60，滚轮后 88×66，双击恢复 80×60；置顶、鼠标拖动锚点及复制源像素保持正确。
- 在原有 Windows 10 构建机完成原生 Release 构建、Clippy 和 **16 项常规单元测试**，均通过；Windows 11 验证机没有安装 Rust 或编译工具链。热键冲突专项、自启动注册专项及独立 OCR 集成测试仍为默认 ignored，本次未宣称在 Windows 11 上单独运行；实际 GUI 热键和随包 OCR 已在桌面流程验证。
- Linux `cargo fmt --check`、Clippy 和 **18 项单元测试通过**，包括新增的 175% DPI 反向框选像素边界回归。

### 本轮修复

1. **小贴图被系统最小窗口高度拉伸**：175% 下 80×60 实际变成 80×64。无边框图像窗口改为 `WS_POPUP`，不再继承 overlapped 窗口的 DPI 缩放最小尺寸限制。
2. **选区多出 1 像素**：175% 下反向选择 173×97 导出为 174×97。输入坐标换算后按源像素对齐，避免浮点误差被裁剪 floor/ceil 放大；新增单元及桌面像素回归。
3. **贴图跨 DPI 屏幕被缩放**：80×60 从 175% 拖到 100% 后变成 46×34。处理 `WM_GETDPISCALEDSIZE`，让系统移动循环在 DPI 改变前保留物理尺寸及拖动锚点，而非仅依赖渲染时二次调整。

### 发行包与最终安装

- 重新生成安装程序和 ZIP，传输前后 SHA-256 一致。安装版和含空格路径的 ZIP 版**各完成 15 项基础检查与 14 项多屏检查**；测试安装的卸载验证通过，程序、卸载项及开始菜单入口被正确移除。
- 正式安装到当前用户 `%LOCALAPPDATA%\Programs\FreePinPaste`，开始菜单入口存在；安装 EXE 与通过测试的 ZIP EXE SHA-256 相同：`1c67152920285e1ae8f8341ddc0dfa712e5711cd7f465fa48d877da5d2ffe97a`。
- 该机器原有 Snipaste 占用 F1 / F3（RegisterHotKey 返回 1409）；经用户明确要求卸载当前用户的 Store 包并退出其残留进程后，**默认 F1 截图 / F3 贴图真实按键验证通过**。未将测试用 Ctrl+Shift+F8 / F9 写成正式配置。
- 最终安装再次通过 14 项多屏检查，完成后保留 FreePinPaste 在交互桌面后台运行；登录自启动保持关闭。未启用电视，也未改变两屏布局与缩放。
- 临时计划任务清理完成；隔离 Python、Pillow、测试解压目录及远端测试材料在证据归档后删除，正式安装保留。

基础与多屏测试会临时切换本应用快捷键、贴图关闭方式并恢复原配置。证据存于开发机 `.cache/windows11-validation/`，包括 `installed-result.json`、`installed-multimon-result.json`、`portable-result.json`、`portable-multimon-result.json`、`package-result.json`、`final-multimon-result.json` 和 `final-install-result.json`；桌面截图不提交 Git。发行包内的验证文档记录打包时状态，此节补记随后的包验收与最终交付结果。

## 未验收项

- **其他多屏配置**：本轮只验收以上双屏 175% / 100% 布局；负横坐标、150% / 200% 组合、三屏及更多活动屏未实机验证。
- 登录后的真实自启动周期：已验证 HKCU Run 项的写入、引用路径和移除，未重启或注销用户桌面。
- Explorer 重启后的托盘重建已实现，未为测试而重启用户 Explorer。
- HDR 色彩管理、旋转屏幕、屏幕热插拔、远程会话切换未验收；Windows 11 验证范围仅限上述 Build 和显示配置。
