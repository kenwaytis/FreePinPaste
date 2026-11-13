#!/usr/bin/env python3
"""Interactive physical-pixel regression tests. Never changes display settings.

Run through run-windows-desktop.ps1 -Arguments '--multimon --exe ...'.
A separate process displays generated reference patterns. Captures can still
include desktop content and must not be committed. Only fixture windows owned
by that process are closed.
"""
import ctypes as c
from ctypes import wintypes as w
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import time

# The smoke runner already configured per-monitor-v2 DPI awareness.
_spec = importlib.util.spec_from_file_location('smoke', Path(__file__).with_name('windows-smoke.py'))
s = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(s)
from PIL import Image, ImageChops, ImageDraw


def pattern(width, height, index=0):
    image = Image.new('RGB', (width, height), (235, 241, 247))
    draw = ImageDraw.Draw(image)
    for y in range(0, height, 31):
        for x in range(0, width, 37):
            color = ((x * 3 + index * 59) % 220, (y * 5 + 41) % 220, (x + y + 73) % 220)
            draw.rectangle((x, y, x + 34, y + 28), fill=color)
    return image


def fixture_desktop():
    u = s.u
    u.CreateWindowExW.argtypes = [w.DWORD, w.LPCWSTR, w.LPCWSTR, w.DWORD, c.c_int, c.c_int,
                                 c.c_int, c.c_int, w.HWND, w.HMENU, w.HINSTANCE, c.c_void_p]
    u.CreateWindowExW.restype = w.HWND
    u.LoadImageW.argtypes = [w.HINSTANCE, w.LPCWSTR, w.UINT, c.c_int, c.c_int, w.UINT]
    u.LoadImageW.restype = w.HANDLE
    u.SendMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
    u.SendMessageW.restype = w.LPARAM
    u.ShowWindow.argtypes = [w.HWND, c.c_int]
    handles = []
    for i, monitor in enumerate(s.inventory()):
        left, top, right, bottom = monitor['bounds']
        path = s.OUT / f'display-pattern-{i}.bmp'
        pattern(right - left, bottom - top, i).save(path)
        bitmap = u.LoadImageW(None, str(path), 0, 0, 0, 0x10)
        hwnd = u.CreateWindowExW(0x80, 'STATIC', 'FreePinPaste validation fixture',
                                 0x80000000 | 0x0E, left, top, right - left, bottom - top,
                                 None, None, None, None)
        if not bitmap or not hwnd:
            raise c.WinError(c.get_last_error())
        u.SendMessageW(hwnd, 0x172, 0, bitmap)  # STM_SETIMAGE / IMAGE_BITMAP
        # Request topmost fixtures; subsequently opened topmost editor/pin
        # windows must remain above them.
        u.SetWindowPos(hwnd, -1, left, top, right - left, bottom - top, 0x10 | 0x40)
        handles.append(hwnd)
    (s.OUT / 'fixture-ready.json').write_text(json.dumps(handles))
    message = w.MSG()
    while u.GetMessageW(c.byref(message), None, 0, 0) > 0:
        u.TranslateMessage(c.byref(message))
        u.DispatchMessageW(c.byref(message))


def run(exe):
    checks = []
    monitors = s.inventory()
    assert len(monitors) >= 2, 'Multi-monitor validation requires at least two desktop displays'
    desktop = [min(m['bounds'][0] for m in monitors), min(m['bounds'][1] for m in monitors),
               max(m['bounds'][2] for m in monitors), max(m['bounds'][3] for m in monitors)]
    ready = s.OUT / 'fixture-ready.json'
    ready.unlink(missing_ok=True)
    fixture = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), '--fixture-desktop'])
    app = None
    log = open(s.OUT / 'multimon-app.log', 'w', encoding='utf-8')

    def command(*args):
        subprocess.run([str(exe), *map(str, args)], check=True, timeout=15)

    def visible(title):
        return next((win for win in s.windows(app.pid) if title in win['title'] and win['visible']), None)

    def snapshot():
        return s.ImageGrab.grab(all_screens=True).convert('RGB')

    def crop(image, box):
        return image.crop((box[0] - desktop[0], box[1] - desktop[1], box[2] - desktop[0], box[3] - desktop[1]))

    def copy_selection(box, name, quick=False, reverse=False):
        # Sample the desktop immediately before capture. The cursor is not part
        # of ImageGrab/GDI output; source content must remain still for comparison.
        s.u.SetCursorPos(box[0] + 5, box[1] + 5)
        time.sleep(0.25)
        reference = snapshot()
        command('--capture-copy' if quick else '--capture')
        s.wait_for(lambda: visible('截图编辑'))
        time.sleep(0.6)
        editor = visible('截图编辑')
        assert editor['bounds'] == desktop, editor
        s.u.SetForegroundWindow(editor['hwnd'])
        a, b = tuple(box[:2]), tuple(box[2:])
        s.drag(b if reverse else a, a if reverse else b)
        toolbar = None
        if not quick:
            frame = snapshot()
            red, green, blue = frame.split()
            mask = ImageChops.multiply(red.point(lambda v: 255 if v == 255 else 0),
                                      green.point(lambda v: 255 if v == 253 else 0))
            mask = ImageChops.multiply(mask, blue.point(lambda v: 255 if v == 247 else 0))
            bounds = mask.getbbox()
            assert bounds, (name, 'toolbar not visible')
            toolbar = [bounds[0] + desktop[0], bounds[1] + desktop[1],
                       bounds[2] + desktop[0], bounds[3] + desktop[1]]
            scale = editor['dpi'] / 96
            assert toolbar[2] - toolbar[0] >= 500 * scale, (name, 'toolbar clipped', toolbar)
            assert toolbar[3] - toolbar[1] >= 40 * scale, (name, 'toolbar clipped', toolbar)
            assert any(m['bounds'][0] <= toolbar[0] and m['bounds'][1] <= toolbar[1]
                       and m['bounds'][2] >= toolbar[2] and m['bounds'][3] >= toolbar[3]
                       for m in monitors), (name, 'toolbar outside active displays', toolbar)
            frame.save(s.OUT / f'{name}-ui.png')
            if name.endswith('-edge'):
                # Exercise the rightmost Copy button, not just its key binding.
                s.click((round(toolbar[2] - 24 * scale), (toolbar[1] + toolbar[3]) // 2))
            else:
                s.key(0x0D)
        s.wait_for(lambda: not visible('截图编辑'))
        actual = s.ImageGrab.grabclipboard().convert('RGB')
        expected = crop(reference, box)
        assert actual.size == expected.size, (name, actual.size, expected.size)
        difference = ImageChops.difference(actual, expected)
        if difference.getbbox():
            actual.save(s.OUT / f'{name}-actual.png')
            expected.save(s.OUT / f'{name}-expected.png')
            difference.save(s.OUT / f'{name}-diff.png')
            raise AssertionError((name, 'captured pixels differ', difference.getbbox()))
        checks.append(dict(check=name, bounds=box, size=actual.size, pixels='exact', toolbar=toolbar))

    try:
        s.wait_for(ready.exists)
        app = subprocess.Popen([str(exe), '--background'], stdout=log, stderr=log)
        s.wait_for(lambda: s.windows(app.pid))
        for i, monitor in enumerate(monitors):
            l, t, r, b = monitor['bounds']
            x, y = (l + r) // 2 - 210, (t + b) // 2 - 110
            copy_selection([x, y, x + 420, y + 220], f'display-{i}-capture')
            copy_selection([x, y, x + 173, y + 97], f'display-{i}-reverse-quick', quick=True, reverse=True)
            # Cover the screen corner adjacent to desktop gaps without placing
            # an end point outside the active display.
            copy_selection([r - 240, b - 190, r - 20, b - 50], f'display-{i}-edge')

        seams = 0
        for i, a in enumerate(monitors):
            for b in monitors[i + 1:]:
                al, at, ar, ab = a['bounds']
                bl, bt, br, bb = b['bounds']
                left, right = max(al, bl), min(ar, br)
                top, bottom = max(at, bt), min(ab, bb)
                box = None
                if right - left >= 420 and (ab == bt or bb == at):
                    seam = ab if ab == bt else bb
                    x = (left + right) // 2 - 210
                    box = [x, seam - 140, x + 420, seam + 140]
                elif bottom - top >= 280 and (ar == bl or br == al):
                    seam = ar if ar == bl else br
                    y = (top + bottom) // 2 - 140
                    box = [seam - 210, y, seam + 210, y + 280]
                if box:
                    copy_selection(box, f'cross-screen-{seams}')
                    copy_selection(box, f'cross-screen-{seams}-reverse', reverse=True)
                    seams += 1
        assert seams, 'No adjacent screen seam covered; adapt test for this layout'

        # Synthetic source checks pin geometry and clipboard data independently
        # of captured desktop pixels, including Windows' minimum tracking size.
        pin_image = s.OUT / 'pin-pattern.png'
        pattern(80, 60).save(pin_image)
        path = str(pin_image).replace("'", "''")
        s.native_clipboard(f"$i=[System.Drawing.Image]::FromFile('{path}'); [System.Windows.Forms.Clipboard]::SetImage($i); $i.Dispose()")
        centers = [((m['bounds'][0] + m['bounds'][2]) // 2, (m['bounds'][1] + m['bounds'][3]) // 2) for m in monitors]
        for i, center in enumerate(centers):
            s.u.SetCursorPos(*center)
            command('--pin')
            pin = s.wait_for(lambda: visible('贴图'))
            time.sleep(0.5)
            pin = visible('贴图')
            assert pin['bounds'] == [*center, center[0] + 80, center[1] + 60], pin
            assert pin['topmost']
            s.u.SetForegroundWindow(pin['hwnd'])
            for j, target in enumerate(centers + [center]):
                pin = visible('贴图')
                l, t, r, b = pin['bounds']
                s.drag((l + 20, t + 20), target)
                time.sleep(0.6)
                moved = visible('贴图')
                l, t, r, b = moved['bounds']
                assert [r - l, b - t] == [80, 60], moved
                assert abs(l + 20 - target[0]) <= 6 and abs(t + 20 - target[1]) <= 6, (moved, target)
                assert moved['topmost']
                s.key(ord('C'), control=True)
                actual = s.ImageGrab.grabclipboard().convert('RGB')
                assert not ImageChops.difference(actual, pattern(80, 60)).getbbox()
                s.u.SetCursorPos(l + 25, t + 25)
                s.u.mouse_event(0x0800, 0, 0, 120, 0)
                time.sleep(0.5)
                zoomed = visible('贴图')['bounds']
                assert [zoomed[2] - zoomed[0], zoomed[3] - zoomed[1]] == [88, 66], zoomed
                # Right-click-close config makes double-click restore 1:1 size.
                s.click((l + 25, t + 25))
                s.click((l + 25, t + 25))
                time.sleep(0.4)
                restored = visible('贴图')['bounds']
                assert [restored[2] - restored[0], restored[3] - restored[1]] == [80, 60], restored
                checks.append(dict(check=f'pin-{i}-move-{j}-zoom-restore', window=moved))
                time.sleep(0.5)
            s.key(0x1B)
            s.wait_for(lambda: not visible('贴图'))
        return checks
    finally:
        if app is not None:
            try:
                command('--quit')
                app.wait(timeout=10)
            except Exception:
                app.terminate()
        fixture.terminate()
        fixture.wait(timeout=10)
        log.close()
        (s.OUT / 'multimon-checks.json').write_text(json.dumps(checks, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__ == '__main__':
    if sys.argv[1:] == ['--fixture-desktop']:
        fixture_desktop()
    else:
        raise SystemExit('Use windows-smoke.py --multimon via the desktop runner')
