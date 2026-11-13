#!/usr/bin/env python3
"""Sync sources / run PowerShell via an existing SSH alias; never stores credentials.

  python3 scripts/windows-remote.py sync
  python3 scripts/windows-remote.py exec 'cargo check --locked'

The remote workspace is dedicated to this checkout. Sync does not remove remote
build caches or runtimes. GUI tests must run in the interactive desktop session.
"""
import argparse
import base64
import io
from pathlib import Path
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', default='paidax-work-win')
    parser.add_argument('--directory', default=r'D:\project\FreePinPaste')
    parser.add_argument('operation', choices=['sync', 'exec'])
    parser.add_argument('command', nargs='?', default='')
    args = parser.parse_args()
    ssh = ['ssh', '-T', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', args.host]
    directory = args.directory.replace("'", "''")

    def run(script):
        prefix = "$OutputEncoding = [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new(); $ProgressPreference = 'SilentlyContinue'; $ErrorActionPreference = 'Stop'; "
        encoded = base64.b64encode((prefix + script).encode('utf-16-le')).decode()
        return subprocess.run(ssh + ['powershell -NoLogo -NoProfile -NonInteractive -EncodedCommand ' + encoded])

    if args.operation == 'exec':
        raise SystemExit(run(f"Set-Location '{directory}'; " + args.command).returncode)
    run(f"New-Item -ItemType Directory -Force '{directory}' | Out-Null").check_returncode()
    files = subprocess.check_output(['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT).decode().split('\0')
    data = io.BytesIO()
    with tarfile.open(fileobj=data, mode='w:gz') as archive:
        for name in sorted(set(files)):
            path = ROOT / name
            if name and path.is_file():
                archive.add(path, arcname=name, recursive=False)
    subprocess.run(ssh + [f'tar.exe -xzf - -C "{args.directory}"'], input=data.getvalue(), check=True)
    print(f'Synced sources to {args.host}:{args.directory}')


if __name__ == '__main__':
    main()
