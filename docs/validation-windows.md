# Windows 验证记录

## 环境

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

## 未验收项

- **多屏、负坐标、混合 DPI**：实现与坐标转换测试已加入，但当前机器只有一块活动屏幕；经用户确认，本次保留为待实机补验，不能以单元测试代替。
- 登录后的真实自启动周期：已验证 HKCU Run 项的写入、引用路径和移除，未重启或注销用户桌面。
- Explorer 重启后的托盘重建已实现，未为测试而重启用户 Explorer。
- HDR、旋转屏幕、屏幕热插拔、远程会话切换和 Windows 11 不在本轮已验证范围内。
