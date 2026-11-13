# 当前环境验收记录

## 环境

- Arch Linux x86_64，GNOME Shell 51.1，Wayland 会话，XWayland `DISPLAY=:0`。
- 三块 1920×1080、100% 缩放显示器横排，完整桌面 5760×1080。
- xdg-desktop-portal 1.22.1、xdg-desktop-portal-gnome 51.0。
- Screenshot v2、GlobalShortcuts v2 均由实际 D-Bus introspection 确认。
- Rust 1.96.0；GPUI 0.2.2；GPUI Component 0.5.1。

## 已验证

| 项目 | 方法 / 结果 |
|---|---|
| Rust 类型检查、编译 | `cargo check`、debug/release 构建，`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 通过 |
| 核心单元测试 | 9 项：选区归一化/边界、工具栏避让、历史分支、橡皮擦还原、PNG 往返、配置、启动项及 OCR 语言校验 |
| 内置 OCR | 实际编译 CPU Tesseract 5.5.0 / Leptonica 1.85.0，不依赖系统 Tesseract；10 个模型 SHA-256 校验通过 |
| 中英混合 OCR | `cargo test --test ocr_smoke -- --ignored --nocapture`，实际得到 `FreePinPaste 2026` 和 `快捷截图 文字识别` |
| F1 / F3 默认绑定 | Portal 返回实际绑定“截图：按 F1 · 置顶：按 F3”，应用监听激活与修改事件 |
| 首次截图授权 | 用户在前台设置窗口点击授权并通过 GNOME 弹窗；PermissionStore 中出现本应用 `yes` |
| 三屏全屏编辑器 | 运行时 X11 geometry 为 `(0,0,5760,1080)`，使用 `_NET_WM_FULLSCREEN_MONITORS` |
| 选区、工具栏、绘图 | 用户实际操作确认选区和工具栏出现，绘图正常 |
| 保存文件 | 用户实际操作确认；通过系统保存对话框生成 PNG |
| 复制图片 | 修复后用户实际粘贴确认成功；另以独立 PNG owner 经 `/usr/bin/wl-paste --type image/png` 回读，与输入 RGBA 逐像素一致 |
| 置顶 | 程序打开剪贴板贴图，X11 实际返回 `_NET_WM_STATE_ABOVE`；无边框 hints 为 decorations=0 |
| 自启动文件 | 临时目录测试启用、移除、重复移除及 `--background` Exec；未擅自给用户启用自启动 |
| 后台窗口 | release 对应代码实测：启动后 keeper 从 `_NET_CLIENT_LIST` 消失，IPC `--quit` 能正常结束进程；设置窗口关闭后进程仍由 keeper 保活 |
| 发行包 | 约 31 MB tar.gz，SHA-256 校验通过；内置 Tesseract/Leptonica/精简 TIFF 静态链接，`ldd` 确认不再依赖系统 libtiff 或其可选编解码器 |
| 安装与搬移 | 临时 HOME 中执行安装脚本，安装后的二进制独立完成中英 OCR；随后经用户确认安装到 `~/.local/share/freepinpaste/app`，release 已运行，日志确认 F1 / F3 绑定，自启动保持关闭 |

## 本轮发现并解决的问题

1. **首次授权 `Other` 错误**：系统日志明确为 `Only the focused app is allowed to show a system access dialog`。改成独立的前台授权步骤，传入本程序 X11 父窗口；授权后丢弃首张截图，再隐藏窗口获取干净截图。权限只读，不直接写授权存储。
2. **GPUI 图片复制为空文本**：GPUI 0.2.2 X11 `write_to_clipboard` 无论数据类型都调用 `set_text`。使用独立、长生命周期 `image/png` selection owner，验证 Mutter 的 X11 → Wayland 剪贴板桥接。
3. **获取原生句柄 panic**：GPUI 的 X11Window `HasWindowHandle` 是 `unimplemented!()`。改为唯一临时标题 + 本进程 PID 定位本程序窗口，不枚举或截取其他程序内容。
4. **后台 1×1 窗口可见**：GPUI X11 不遵守 `show=false`，直接 UnmapWindow 还会和 Mutter 初始 MapRequest 竞争。设置 skip-taskbar / skip-pager，并在初始映射结束后再次隐藏；实测后台 keeper 不再留在客户端窗口列表中。
5. **新桌面入口注册竞态**：刚写入 `.desktop` 时 Portal 的 GAppInfo 缓存未刷新会报 `App info not found`。仅对此已观察到的错误做短暂重试；已有桌面入口不在每次运行时覆盖。
6. **新 libc 与旧 xattr 不兼容**：对传递依赖 xattr 0.2.3 做 Linux `ENOATTR` → `ENODATA` 补丁。

## 未作完成声明的测试

- 当前 GNOME 不接受 XTest 鼠标注入；自动化脚本的“点击/拖动”不作为已验证依据。
- 原生 Wayland 输入法的全部中文候选行为、每个工具参数组合，以及旋转/混合 DPI 显示器尚未做穷举验收。
- 开机自启动已测试入口文件语义，未实际注销或重启用户桌面。
- OCR 按钮的结果编辑流程可按下面清单手动补验；后端真实 OCR 已单独运行验证。

## 后续手动回归清单

- [ ] 用系统改键把 F1 改成其他键，确认设置页实际绑定更新，旧键不再触发。
- [ ] 两个显示器之间跨屏框选，选区靠近屏幕底部/右侧，检查工具栏不越界。
- [ ] 矩形/椭圆分别测试空心、填充；箭头/直线、画笔分别测试颜色和线宽。
- [ ] 擦除标注，撤销、重做；撤销后重新绘制，确认旧分支不能再重做。
- [ ] 输入中英文本，调整粗体/斜体/字号，比较显示与导出。
- [ ] 截取中英文字运行 OCR，编辑识别文本后复制；OCR 过程中 Esc 能退出截屏。
- [ ] 取消保存对话框后，编辑内容仍在；保存一个已有文件时系统明确询问替换。
- [ ] 贴图拖动、滚轮缩放、双击原始尺寸、右键关闭。
- [ ] 安装到稳定路径后启用自启动，下一次登录桌面确认后台驻留。
