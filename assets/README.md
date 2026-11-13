# FreePinPaste 图标

视觉方向：蓝青色「截图框＋图钉」。

- `logo-source.png`：用户通过外部生图工具交付的原始白底 PNG。
- `logo.png`：去除白底、清理背景纹理、统一配色后的透明 512×512 应用图标。
- `tray-symbolic.svg`：从相同轮廓描摹的单色托盘图标，减少留白以适配小尺寸；支持 symbolic 图标的宿主可按面板主题着色。

图标编译进程序；启动及 `--install-desktop` 会把图标写入用户的 `icons/hicolor` 目录，无需在发行包外另行寻找资源。不能使用 symbolic 图标的托盘宿主可回退到程序提供的彩色 ARGB 图标。

重新生成（只用于开发，不是运行/构建依赖）：

```bash
# 需要 Python Pillow 与 potrace
python3 scripts/prepare-icons.py
```
