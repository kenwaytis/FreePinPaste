#!/usr/bin/env python3
"""Interactive Windows smoke checks. Requires Python + Pillow on the dev machine.
Run using run-windows-desktop.ps1, never directly in SSH's Session 0.
Only FreePinPaste-owned processes/windows are controlled. Display settings are
read-only. Tests intentionally use the mouse, keyboard and image clipboard.
"""
import argparse
import base64
import ctypes as c
from ctypes import wintypes as w
import json
import os
import re
from pathlib import Path
import subprocess
import time
import traceback

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / '.cache' / 'windows-validation'
OUT.mkdir(parents=True, exist_ok=True)
u = c.WinDLL('user32', use_last_error=True)
u.SetProcessDpiAwarenessContext.argtypes = [c.c_void_p]
u.SetProcessDpiAwarenessContext(c.c_void_p(-4))
from PIL import Image, ImageGrab  # noqa: E402

u.GetWindowTextW.argtypes = [w.HWND, w.LPWSTR, c.c_int]
u.GetWindowRect.argtypes = [w.HWND, c.POINTER(w.RECT)]
u.IsWindowVisible.argtypes = [w.HWND]
u.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
u.GetWindowLongW.argtypes = [w.HWND, c.c_int]
u.GetDpiForWindow.argtypes = [w.HWND]
u.SetForegroundWindow.argtypes = [w.HWND]
u.PostMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
u.SetWindowPos.argtypes = [w.HWND, w.HWND, c.c_int, c.c_int, c.c_int, c.c_int, w.UINT]


def inventory():
    class Monitor(c.Structure):
        _fields_ = [('size', w.DWORD), ('monitor', w.RECT), ('work', w.RECT), ('flags', w.DWORD), ('device', w.WCHAR * 32)]
    info = []
    callback_type = c.WINFUNCTYPE(w.BOOL, w.HMONITOR, w.HDC, c.POINTER(w.RECT), w.LPARAM)
    u.GetMonitorInfoW.argtypes = [w.HMONITOR, c.POINTER(Monitor)]
    sh = c.WinDLL('shcore')
    sh.GetDpiForMonitor.argtypes = [w.HMONITOR, c.c_int, c.POINTER(w.UINT), c.POINTER(w.UINT)]
    @callback_type
    def collect(handle, dc, rect, data):
        monitor = Monitor(size=c.sizeof(Monitor))
        u.GetMonitorInfoW(handle, c.byref(monitor))
        x, y = w.UINT(), w.UINT()
        sh.GetDpiForMonitor(handle, 0, c.byref(x), c.byref(y))
        r = rect.contents
        info.append(dict(device=monitor.device, bounds=[r.left, r.top, r.right, r.bottom], dpi=[x.value, y.value], primary=bool(monitor.flags & 1)))
        return True
    u.EnumDisplayMonitors(None, None, collect, 0)
    return info


def windows(pid=None):
    result = []
    callback_type = c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    @callback_type
    def collect(handle, data):
        owner = w.DWORD()
        u.GetWindowThreadProcessId(handle, c.byref(owner))
        text = c.create_unicode_buffer(512)
        u.GetWindowTextW(handle, text, 512)
        if (pid is None and text.value.startswith('FreePinPaste')) or (pid == owner.value):
            rect = w.RECT()
            u.GetWindowRect(handle, c.byref(rect))
            result.append(dict(hwnd=handle, pid=owner.value, title=text.value, visible=bool(u.IsWindowVisible(handle)),
                               bounds=[rect.left, rect.top, rect.right, rect.bottom], dpi=u.GetDpiForWindow(handle),
                               topmost=bool(u.GetWindowLongW(handle, -20) & 8)))
        return True
    u.EnumWindows(collect, 0)
    return result


def wait_for(test, timeout=12):
    start = time.monotonic()
    while time.monotonic() - start < timeout:
        value = test()
        if value:
            return value
        time.sleep(0.1)
    raise AssertionError('Timed out waiting for desktop state')


def key(code, control=False, shift=False, alt=False):
    if control:
        u.keybd_event(0x11, 0, 0, 0)
    if shift:
        u.keybd_event(0x10, 0, 0, 0)
    if alt:
        u.keybd_event(0x12, 0, 0, 0)
    u.keybd_event(code, 0, 0, 0)
    time.sleep(0.03)
    u.keybd_event(code, 0, 2, 0)
    if alt:
        u.keybd_event(0x12, 0, 2, 0)
    if shift:
        u.keybd_event(0x10, 0, 2, 0)
    if control:
        u.keybd_event(0x11, 0, 2, 0)
    time.sleep(0.3)


def drag(start, end):
    u.SetCursorPos(*start)
    u.mouse_event(2, 0, 0, 0, 0)
    time.sleep(0.1)  # Allow the native system move loop to start.
    for i in range(1, 16):
        u.SetCursorPos(round(start[0] + (end[0] - start[0]) * i / 15), round(start[1] + (end[1] - start[1]) * i / 15))
        time.sleep(0.025)
    u.mouse_event(4, 0, 0, 0, 0)
    time.sleep(0.4)


def native_clipboard(script):
    script = 'Add-Type -AssemblyName System.Windows.Forms; ' + script
    encoded = base64.b64encode(script.encode('utf-16-le')).decode()
    subprocess.run(['powershell', '-NoProfile', '-STA', '-NonInteractive', '-EncodedCommand', encoded],
                   check=True, timeout=15, creationflags=0x08000000)


def click(point):
    u.SetCursorPos(*point)
    u.mouse_event(2, 0, 0, 0, 0)
    time.sleep(0.08)
    u.mouse_event(4, 0, 0, 0, 0)
    time.sleep(0.12)


def smoke(exe):
    output = open(OUT / 'app.log', 'w', encoding='utf-8')
    environment = os.environ.copy()
    environment.pop('FREEPINPASTE_OCR_DIR', None)
    app = subprocess.Popen([str(exe), '--settings'], stdout=output, stderr=output, cwd=ROOT, env=environment)
    checks = []
    def command(*args):
        subprocess.run([str(exe), *args], check=True, timeout=10, env=environment)
    def visible(title):
        return next((win for win in windows() if title in win['title'] and win['visible']), None)
    try:
        settings = wait_for(lambda: visible('设置'))
        checks.append(dict(check='settings', window=settings))
        time.sleep(1)
        u.SetForegroundWindow(settings['hwnd'])
        time.sleep(0.5)
        ImageGrab.grab(all_screens=True).save(OUT / 'settings.png')
        command('--settings')
        assert len([win for win in windows() if '设置' in win['title'] and win['visible']]) == 1
        checks.append(dict(check='single_instance', passed=True))
        command('--capture')
        editor = wait_for(lambda: visible('截图编辑'))
        time.sleep(1)
        ImageGrab.grab(all_screens=True).save(OUT / 'capture.png')
        displays = inventory()
        expected = [min(d['bounds'][0] for d in displays), min(d['bounds'][1] for d in displays),
                    max(d['bounds'][2] for d in displays), max(d['bounds'][3] for d in displays)]
        assert editor['bounds'] == expected, (editor, expected)
        checks.append(dict(check='physical_desktop_overlay', window=editor))
        monitor = next(d for d in displays if d['primary'])['bounds']
        start = (monitor[0] + 200, monitor[1] + 180)
        end = (start[0] + 420, start[1] + 220)
        u.SetForegroundWindow(editor['hwnd'])
        drag(start, end)
        ImageGrab.grab(all_screens=True).save(OUT / 'selection.png')
        # Use temporary Ctrl+Shift+F8/F9 bindings: defaults may belong to Snipaste.
        # These are real registered hotkeys, not IPC substitutes.
        key(0x78, control=True, shift=True)
        pin = wait_for(lambda: visible('贴图'))
        assert pin['topmost'], pin
        assert pin['bounds'] == [*start, *end], (pin, start, end)
        checks.append(dict(check='hotkey_pin_geometry_topmost', window=pin))
        u.SetForegroundWindow(pin['hwnd'])
        key(ord('C'), control=True)
        clip = ImageGrab.grabclipboard()
        assert isinstance(clip, Image.Image), type(clip)
        assert clip.size == (420, 220), clip.size
        clip.save(OUT / 'copied.png')
        checks.append(dict(check='native_image_clipboard', size=clip.size))
        drag((start[0] + 30, start[1] + 30), (start[0] + 90, start[1] + 70))
        moved = wait_for(lambda: visible('贴图'))
        # Native caption dragging has a system drag threshold; capture/pin
        # placement above is exact, while drag initiation may consume a few px.
        assert abs(moved['bounds'][0] - start[0] - 60) <= 5 and abs(moved['bounds'][1] - start[1] - 40) <= 5, moved
        u.mouse_event(0x0800, 0, 0, 120, 0)
        time.sleep(0.5)
        zoomed = visible('贴图')
        assert [zoomed['bounds'][2] - zoomed['bounds'][0], zoomed['bounds'][3] - zoomed['bounds'][1]] == [462, 242], zoomed
        checks.append(dict(check='pin_drag_zoom', window=zoomed))
        key(0x1B)
        wait_for(lambda: not visible('贴图'))
        external = OUT / 'external-clipboard.png'
        Image.new('RGB', (80, 60), (31, 139, 211)).save(external)
        path = str(external).replace("'", "''")
        native_clipboard(f"$i=[System.Drawing.Image]::FromFile('{path}'); [System.Windows.Forms.Clipboard]::SetImage($i); $i.Dispose()")
        command('--pin')
        external_pin = wait_for(lambda: visible('贴图'))
        assert [external_pin['bounds'][2] - external_pin['bounds'][0], external_pin['bounds'][3] - external_pin['bounds'][1]] == [80, 60], external_pin
        click((external_pin['bounds'][0] + 20, external_pin['bounds'][1] + 20))
        click((external_pin['bounds'][0] + 20, external_pin['bounds'][1] + 20))
        wait_for(lambda: not visible('贴图'))
        checks.append(dict(check='external_bitmap_pin_double_click_close', passed=True))
        # Capture hotkey -> select -> Enter copies and closes.
        key(0x77, control=True, shift=True)
        editor = wait_for(lambda: visible('截图编辑'))
        time.sleep(0.5)
        u.SetForegroundWindow(editor['hwnd'])
        drag(start, end)
        key(0x0D)
        wait_for(lambda: not visible('截图编辑'))
        checks.append(dict(check='hotkey_capture_enter_copy', passed=True))
        command('--capture')
        editor = wait_for(lambda: visible('截图编辑'))
        time.sleep(0.5)
        drag(start, end)
        key(ord('S'), control=True)
        def dialog():
            return next((win for win in windows(app.pid) if win['title'] == '保存截图' and win['visible']), None)
        save = wait_for(dialog)
        u.SetForegroundWindow(save['hwnd'])
        ImageGrab.grab(all_screens=True).save(OUT / 'save-dialog.png')
        key(0x1B)
        wait_for(lambda: visible('截图编辑'))
        checks.append(dict(check='save_cancel_restores_editor', passed=True))
        key(ord('S'), control=True)
        save = wait_for(dialog)
        u.SetForegroundWindow(save['hwnd'])
        saved = OUT / 'saved-中文.png'
        saved.unlink(missing_ok=True)
        path = str(saved).replace("'", "''")
        native_clipboard(f"[System.Windows.Forms.Clipboard]::SetText('{path}')")
        key(ord('N'), alt=True)
        key(ord('A'), control=True)
        key(ord('V'), control=True)
        key(0x0D)
        wait_for(saved.exists)
        wait_for(lambda: not visible('截图编辑'))
        with Image.open(saved) as image:
            assert image.size == (420, 220), image.size
        checks.append(dict(check='native_save_unicode_path', passed=True))
        fixture = OUT / 'white-fixture.png'
        Image.new('RGB', (expected[2] - expected[0], expected[3] - expected[1]), 'white').save(fixture)
        command('--edit', str(fixture))
        editor = wait_for(lambda: visible('截图编辑'))
        time.sleep(0.5)
        drag(start, (start[0] + 700, start[1] + 420))
        key(ord('T'))
        click((start[0] + 40, start[1] + 60))
        ImageGrab.grab(all_screens=True).save(OUT / 'text-panel.png')
        native_clipboard("[System.Windows.Forms.Clipboard]::SetText('FreePinPaste 中文 2026')")
        key(ord('V'), control=True)
        ImageGrab.grab(all_screens=True).save(OUT / 'text-pasted.png')
        key(0x0D, control=True)
        ImageGrab.grab(all_screens=True).save(OUT / 'text-committed.png')
        key(ord('R'))
        a, b = (start[0] + 100, start[1] + 170), (start[0] + 400, start[1] + 320)
        drag(a, b)
        sample = (start[0] + 250, start[1] + 170)
        assert ImageGrab.grab().getpixel(sample)[:3] != (255, 255, 255)
        key(ord('Z'), control=True)
        assert ImageGrab.grab().getpixel(sample)[:3] == (255, 255, 255)
        key(ord('Y'), control=True)
        assert ImageGrab.grab().getpixel(sample)[:3] != (255, 255, 255)
        key(ord('C'), control=True)
        wait_for(lambda: not visible('截图编辑'))
        annotated = OUT / 'annotated.png'
        ImageGrab.grabclipboard().save(annotated)
        ocr = subprocess.run([str(exe), '--ocr', str(annotated)], capture_output=True, encoding='utf-8', check=True, timeout=100, env=environment)
        assert 'FreePinPaste' in ocr.stdout and '2026' in ocr.stdout, ocr.stdout
        checks.append(dict(check='text_rectangle_undo_redo_copy_ocr', text=ocr.stdout.strip()))
        command('--capture-copy')
        editor = wait_for(lambda: visible('截图编辑'))
        time.sleep(0.5)
        drag(start, (start[0] + 180, start[1] + 120))
        wait_for(lambda: not visible('截图编辑'))
        assert ImageGrab.grabclipboard().size == (180, 120)
        checks.append(dict(check='quick_capture', passed=True))
        command('--restart')
        app.wait(timeout=15)
        wait_for(lambda: any(win['pid'] != app.pid for win in windows()))
        tray = wait_for(lambda: next((win for win in windows() if win['title'] == 'FreePinPaste.Desktop'), None))
        u.PostMessageW(tray['hwnd'], 0x8001, 1, 0x205)
        u.FindWindowW.argtypes = [w.LPCWSTR, w.LPCWSTR]
        u.FindWindowW.restype = w.HWND
        menu = wait_for(lambda: u.FindWindowW('#32768', None))
        assert menu
        ImageGrab.grab(all_screens=True).save(OUT / 'tray-menu.png')
        rect = w.RECT()
        u.GetWindowRect(menu, c.byref(rect))
        # Select the third of the five native menu items with a real mouse click.
        click(((rect.left + rect.right) // 2, (rect.top + rect.bottom) // 2))
        restarted = wait_for(lambda: visible('设置'))
        checks.append(dict(check='tray_menu_settings', passed=True))
        assert restarted['pid'] != app.pid
        checks.append(dict(check='background_restart', window=restarted))
        command('--quit')
        wait_for(lambda: not windows())
        assert ImageGrab.grabclipboard().size == (180, 120)
        checks.append(dict(check='clipboard_survives_exit', passed=True))
        return checks
    finally:
        try:
            command('--quit')
        except Exception:
            pass
        if app.poll() is None:
            app.terminate()
        output.close()
        (OUT / 'checks.json').write_text(json.dumps(checks, ensure_ascii=False, indent=2), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--inventory', action='store_true')
    parser.add_argument('--hotkey-test-only', action='store_true')
    parser.add_argument('--exe', type=Path, default=ROOT / 'target' / 'debug' / 'freepinpaste.exe')
    args = parser.parse_args()
    (OUT / 'failure.txt').unlink(missing_ok=True)
    (OUT / 'result.json').unlink(missing_ok=True)
    result = dict(monitors=inventory(), windows=windows())
    (OUT / 'inventory.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    if args.hotkey_test_only:
        cargo = Path.home() / '.cargo' / 'bin' / 'cargo.exe'
        with open(OUT / 'hotkey-test.log', 'wb') as log:
            subprocess.run([str(cargo), 'test', '--locked', '--lib', 'hotkey_conflict_keeps_old_bindings', '--', '--ignored', '--nocapture'],
                           cwd=ROOT, stdout=log, stderr=log, check=True, timeout=120, creationflags=0x08000000)
        result['hotkey_transaction_test'] = 'passed'
    elif not args.inventory:
        if windows():
            raise RuntimeError('An existing FreePinPaste instance is running; close it before validation')
        config = Path(os.environ['APPDATA']) / 'freepinpaste' / 'config.toml'
        original = config.read_bytes() if config.exists() else None
        text = original.decode('utf-8') if original else ''
        for name, value in [('capture_shortcut', 'Ctrl+Shift+F8'), ('pin_shortcut', 'Ctrl+Shift+F9')]:
            text = re.sub(rf'^{name}\s*=.*\n?', '', text, flags=re.M)
            text += f'\n{name} = "{value}"\n'
        config.parent.mkdir(parents=True, exist_ok=True)
        try:
            config.write_text(text, encoding='utf-8')
            result['checks'] = smoke(args.exe)
        finally:
            if original is None:
                config.unlink(missing_ok=True)
            else:
                config.write_bytes(original)
    (OUT / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__ == '__main__':
    try:
        main()
    except Exception:
        (OUT / 'failure.txt').write_text(traceback.format_exc(), encoding='utf-8')
        raise
