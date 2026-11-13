#!/usr/bin/env python3
"""Prepare the generated blue/cyan-on-white logo. Requires Pillow and potrace."""
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
source = Image.open(ASSETS / "logo-source.png").convert("RGB")
logo = Image.new("RGBA", source.size)
pixels = []
source_pixels = source.load()
for r, g, b in (source_pixels[x, y] for y in range(source.height) for x in range(source.width)):
    # The generated mark has no white/gray foreground. Unmatte white edges,
    # suppress near-white background texture, and flatten the two brand colors.
    alpha = 255 - r if max(r, g, b) - min(r, g, b) > 35 else 0
    cyan = g - r > (b - r) * 0.55
    pixels.append((0, 181, 220, alpha) if cyan else (0, 96, 220, alpha))
logo.putdata(pixels)
app_icon = logo.resize((512, 512), Image.Resampling.LANCZOS)
app_icon.save(ASSETS / "logo.png")
app_icon.save(ASSETS / "freepinpaste.ico", sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])

# Use less padding on the small tray icon than on the launcher icon.
mask = logo.getchannel("A")
mask = mask.crop(mask.getbbox())
side = max(mask.size)
canvas = Image.new("L", (side + side // 8, side + side // 8))
canvas.paste(mask, ((canvas.width - mask.width) // 2, (canvas.height - mask.height) // 2))
mask = canvas.resize((256, 256), Image.Resampling.LANCZOS)
with tempfile.TemporaryDirectory() as temp:
    bitmap = Path(temp) / "tray.pbm"
    mask.point(lambda a: 0 if a >= 128 else 255).convert("1").save(bitmap)
    subprocess.run([
        "potrace", str(bitmap), "--svg", "--flat", "--color", "#2e3436",
        "--output", str(ASSETS / "tray-symbolic.svg"),
    ], check=True)
print("Prepared assets/logo.png, assets/freepinpaste.ico and assets/tray-symbolic.svg")
