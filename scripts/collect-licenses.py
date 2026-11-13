#!/usr/bin/env python3
"""Collect Cargo dependency license files and an exact version/license inventory."""
import json
import pathlib
import shutil
import subprocess
import sys

out = pathlib.Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)
target = sys.argv[2] if len(sys.argv) > 2 else 'x86_64-unknown-linux-gnu'
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version=1", f"--filter-platform={target}"
]))
inventory = []
for package in metadata["packages"]:
    if package["name"] == "freepinpaste":
        continue
    root = pathlib.Path(package["manifest_path"]).parent
    target = out / f'{package["name"]}-{package["version"]}'
    candidates = [p for p in root.iterdir() if p.is_file() and p.name.lower().startswith(("license", "licence", "copying", "notice", "copyright"))]
    if candidates:
        target.mkdir(exist_ok=True)
        for source in candidates:
            shutil.copy2(source, target / source.name)
    inventory.append({k: package.get(k) for k in ("name", "version", "license", "license_file", "repository", "source")})
(out / "cargo-dependencies.json").write_text(json.dumps(inventory, ensure_ascii=False, indent=2), encoding='utf-8')
