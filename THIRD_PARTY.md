# Third-party components

FreePinPaste source is licensed under Apache-2.0. Dependency licenses remain their respective licenses; an exact Rust dependency/license inventory and available upstream license files are collected into release packages by `scripts/collect-licenses.py`.

## Bundled OCR

| Component | Fixed version / commit | License | Source |
|---|---|---|---|
| OCRia-nano-1 | v1.0.0 | OCRia-nano-1 Model License | https://huggingface.co/llamaindex/OCRia-nano-1 |
| LiteParse native CLI and Rust library (local OCR worker) | 2.21.0 | Apache-2.0 | https://github.com/run-llama/liteparse/releases/tag/crates-v2.21.0 |
| PDFium | chromium/8172 | BSD and third-party notices | https://github.com/run-llama/pdfium-binaries |
| Microsoft ONNX Runtime | 1.31.1 | MIT and third-party notices | https://www.nuget.org/packages/Microsoft.ML.OnnxRuntime/1.31.1 |

The unmodified model and its metadata are by LlamaIndex, Inc. Their use and redistribution are subject to the complete model license at `ocr/models/ocria-nano/LICENSE`, **including Section 5 and Attachment A**. The Apache-2.0 license for FreePinPaste and LiteParse does not grant rights to these weights. See the [model license](https://huggingface.co/llamaindex/OCRia-nano-1/blob/main/LICENSE) and [licensing contact](https://www.llamaindex.ai/contact).

`packaging/models.sha256` pins the model files and license. `scripts/prepare-ocr.py` verifies the native archives and collects notices into `ocr/licenses/`. Linux uses the system C/C++ runtime. Windows packages include app-local x64 MSVC redistributable DLLs from the builder's licensed Visual Studio installation ([Microsoft redistribution terms](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files)).

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
