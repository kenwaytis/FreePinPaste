# Third-party components

FreePinPaste source is licensed under Apache-2.0. Dependency licenses remain their respective licenses; an exact Rust dependency/license inventory and available upstream license files are collected into release packages by `scripts/collect-licenses.py`.

## Bundled OCR

| Component | Fixed version / commit | License | Source |
|---|---|---|---|
| Tesseract | 5.5.0 / `ad3ba9c9c665d879553c27980d868225943ce9f5` | Apache-2.0 | https://github.com/tesseract-ocr/tesseract |
| Leptonica | 1.85.0 / `63aef18d98432b8582a1565e241f7bd2ee9cc8d9` | BSD-2-Clause | https://github.com/DanBloomberg/leptonica |
| libtiff | 4.7.2, archive SHA-256 `672bd7d10aee4606171afb864f3570b83340f6a33e2c186dc0512f7145ffdf6a` | libtiff permissive license | https://download.osgeo.org/libtiff/ |
| tessdata_fast | `87416418657359cb625c412a48b6e1d6d41c29bd` | Apache-2.0 | https://github.com/tesseract-ocr/tessdata_fast |

The Linux build script statically links Tesseract, Leptonica and a minimal libtiff into the OCR executable. Optional TIFF codecs (JBIG, LZMA, ZSTD, WebP, libdeflate) are disabled. The release script bundles the remaining non-glibc shared libraries from the build machine: libpng (libpng license), zlib (Zlib), libjpeg-turbo (BSD/IJG/zlib), and GCC runtime libraries (GPL with Runtime Library Exception). Their installed license texts and the GCC exception are included in `licenses/`. System glibc and graphics drivers are not redistributed.

The Windows OCR build uses the same Tesseract, Leptonica, TIFF and model revisions, and additionally pins:

| Component | Version / archive SHA-256 | License / source |
|---|---|---|
| zlib | 1.3.1 / `17e88863f3600672ab49182f217281b6fc4d3c762bde361935e436a95214d05c` | Zlib — https://github.com/madler/zlib |
| libpng | 1.6.50 / `71158e53cfdf2877bc99bcab33641d78df3f48e6e0daad030afe9cb8c031aa46` | libpng — https://github.com/pnggroup/libpng |
| libjpeg-turbo | 3.1.0 / `35fec2e1ddfb05ecf6d93e50bc57c1e54bc81c16d611ddf6eff73fff266d8285` | BSD/IJG/zlib — https://github.com/libjpeg-turbo/libjpeg-turbo |

Windows links these codecs and the MSVC CRT statically, disables OpenMP and optional TIFF codecs, and does not redistribute graphics drivers or system DLLs. Native dependency license files are included under `ocr/licenses/`.

Model hashes are in `packaging/models.sha256`. Tesseract, Leptonica and tessdata license texts are in the generated `ocr/licenses/` directory. No proprietary OCR runtime or cloud service is included. System fonts are used rather than redistributed.

## GPUI and supporting crates

- GPUI 0.2.2: Apache-2.0 — https://github.com/zed-industries/zed
- GPUI Component 0.5.1: Apache-2.0 — https://github.com/longbridge/gpui-component
- ashpd 0.12.3: MIT — https://github.com/bilelmoussaoui/ashpd
- ksni 0.3.7: Unlicense — https://github.com/iovxw/ksni
- tiny-skia / resvg: see Cargo license inventory — https://github.com/linebender/resvg
- x11-clipboard 0.9.3: MIT — https://github.com/quininer/x11-clipboard
- arboard / clipboard-win: Windows image clipboard — https://github.com/1Password/arboard / https://github.com/DoumanAsh/clipboard-win
- windows-rs / windows-registry: MIT OR Apache-2.0 — https://github.com/microsoft/windows-rs

## Windows installer tooling

The installer is built with Inno Setup 6.4.3 from its official release:
https://github.com/jrsoftware/issrc/releases/tag/is-6_4_3

The optional preparation script pins the installer SHA-256 to `f3c42116542c4cc57263c5ba6c4feabfc49fe771f2f98a79d2f7628b8762723b` and verifies Authenticode before user-scope installation. The build compiler itself is not bundled with FreePinPaste. Inno Setup licensing: https://github.com/jrsoftware/issrc/blob/is-6_4_3/license.txt

## Local compatibility patch

`vendor/xattr` is xattr **0.2.3** (MIT OR Apache-2.0), used transitively by GPUI → gpui_http_client → zed-async-tar. Upstream: https://github.com/Stebalien/xattr

Only the errno definition in `src/sys/mod.rs` is changed: Linux/Android use `libc::ENODATA`, because newer libc versions removed the obsolete `ENOATTR` alias. BSD/macOS behavior is unchanged. Both original license files are preserved. The patch avoids downgrading libc (newer dependencies require newer versions).

GPUI native-handle, image clipboard and Windows 10 borderless-client-area limitations are handled in application code, without vendoring or modifying GPUI itself. Windows uses GPUI's existing PerMonitorV2 manifest plus the application's own icon resource; adding a second manifest would create duplicate resources.
