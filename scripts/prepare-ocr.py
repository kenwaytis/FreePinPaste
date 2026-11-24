"""Assemble the offline x64 OCR runtime from pinned upstream release artifacts."""
import hashlib
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CACHE = ROOT / '.cache/ocria'
OUT = ROOT / 'runtime/ocria'
WIN = sys.platform == 'win32'
TARGET = 'windows' if WIN else 'linux'
MODEL = 'models/ocria-nano'
LIT = 'https://github.com/run-llama/liteparse/releases/download/crates-v2.21.0'
HF = 'https://huggingface.co/llamaindex/OCRia-nano-1/resolve/v1.0.0'
ORT = 'microsoft.ml.onnxruntime.1.31.1.nupkg'


def digest(path):
    with path.open('rb') as f:
        h = hashlib.sha256()
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
        return h.hexdigest()


def fetch(name, url, checksum):
    path = CACHE / name
    if path.is_file() and digest(path) == checksum:
        return path
    partial = path.with_suffix('.part')
    subprocess.run(['curl.exe' if WIN else 'curl', '-fLsS', '--connect-timeout', '20', '--retry', '3', '-o', partial, url], check=True)
    if digest(partial) != checksum:
        raise ValueError(f'SHA-256 mismatch: {name}')
    partial.replace(path)
    return path


def copy_windows_crt(destination):
    # Only use app-local redistributables from VS, never copy DLLs from System32.
    redist = os.environ.get('VCToolsRedistDir')
    if redist:
        candidates = Path(redist).glob('x64/Microsoft.VC*.CRT')
    else:
        vswhere = Path(os.environ['ProgramFiles(x86)']) / 'Microsoft Visual Studio/Installer/vswhere.exe'
        install = subprocess.check_output([vswhere, '-latest', '-products', '*', '-requires',
                                           'Microsoft.VisualStudio.Component.VC.Tools.x86.x64',
                                           '-property', 'installationPath', '-utf8'], encoding='utf-8').strip()
        candidates = (Path(install) / 'VC/Redist/MSVC').glob('*/x64/Microsoft.VC*.CRT')
    crt = next((p for p in sorted(candidates, reverse=True) if (p / 'msvcp140_1.dll').is_file()), None)
    if not crt:
        raise RuntimeError('Missing VS x64 redistributable CRT; set VCToolsRedistDir')
    for name in ('msvcp140.dll', 'msvcp140_1.dll', 'vcruntime140.dll', 'vcruntime140_1.dll'):
        shutil.copyfile(crt / name, destination / name)


def assemble(out):
    model = out / MODEL
    (model / 'runtime').mkdir(parents=True)
    (out / 'bin').mkdir()
    licenses = out / 'licenses'
    licenses.mkdir()
    for line in (ROOT / 'packaging/models.sha256').read_text().splitlines():
        checksum, rel = line.split()
        name = Path(rel).name
        shutil.copyfile(fetch(name, f'{HF}/{name}?download=true', checksum), out / rel)

    bundle = f'lit-{TARGET}-x64'
    checksum = ('26990ee285ac653303e1f68448a837c60f79c0da2a48969c0237093aaee54069' if WIN
                else '17fa11540f0f7203138fba6f1b9c96f6700c630c9a0309525348d659c345817e')
    archive = fetch(bundle + '.tar.gz', f'{LIT}/{bundle}.tar.gz', checksum)
    with tarfile.open(archive) as tar:
        exe, pdf = ('lit.exe', 'pdfium.dll') if WIN else ('lit', 'libpdfium.so')
        for name in (exe, pdf):
            (out / 'bin' / name).write_bytes(tar.extractfile(f'{bundle}/{name}').read())
        (out / 'bin' / exe).chmod(0o755)

    archive = fetch(ORT, f'https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/1.31.1/{ORT}',
                    '6871ccbc97f3d5bdb4095a1dcec33aa78d56f6ed8de5980ef5fa1d9f2e7c679a')
    with zipfile.ZipFile(archive) as z:
        target, lib = ('win', 'onnxruntime.dll') if WIN else ('linux', 'libonnxruntime.so')
        (model / 'runtime' / lib).write_bytes(z.read(f'runtimes/{target}-x64/native/{lib}'))
        for name in ('LICENSE', 'ThirdPartyNotices.txt'):
            (licenses / ('ONNXRuntime-' + name)).write_bytes(z.read(name))

    shutil.copyfile(ROOT / 'packaging/licenses/LiteParse-LICENSE', licenses / 'LiteParse-LICENSE')
    # The CLI tarballs omit the license directory of their PDFium distribution.
    archive = fetch('pdfium-linux-x64.tgz', 'https://github.com/run-llama/pdfium-binaries/releases/download/chromium%2F8172/pdfium-linux-x64.tgz',
                    '8562f2b56bff77b1a9f559807fb4133a3897d5228973b3593cf56fc537c3cb8d')
    with tarfile.open(archive) as tar:
        pdf_licenses = licenses / 'PDFium'
        pdf_licenses.mkdir()
        for member in tar:
            if member.isfile() and (member.name == 'LICENSE' or member.name.startswith('licenses/')):
                (pdf_licenses / Path(member.name).name).write_bytes(tar.extractfile(member).read())

    if WIN:
        copy_windows_crt(out / 'bin')
    subprocess.run([out / 'bin' / exe, '--version'], check=True)


if __name__ == '__main__':
    if platform.machine().lower() not in ('x86_64', 'amd64') or sys.platform not in ('linux', 'win32'):
        sys.exit('FreePinPaste OCR packages support Linux and Windows x64')
    CACHE.mkdir(parents=True, exist_ok=True)
    OUT.parent.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=OUT.parent) as temp:
        staged = Path(temp) / 'ocria'
        assemble(staged)
        if OUT.exists():
            shutil.rmtree(OUT)
        staged.rename(OUT)
    print(f'OCRia-nano-1 ready: {OUT}')
