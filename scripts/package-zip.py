#!/usr/bin/env python3
"""Create a standard ZIP (forward-slash paths, including when built on Windows)."""
import argparse
from pathlib import Path
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
parser.add_argument('archive', type=Path)
args = parser.parse_args()
if not args.directory.is_dir():
    parser.error(f'Not a directory: {args.directory}')

with zipfile.ZipFile(args.archive, 'w', compression=zipfile.ZIP_DEFLATED,
                     compresslevel=9, strict_timestamps=False) as archive:
    for path in sorted(args.directory.rglob('*')):
        if path.is_file():
            archive.write(path, path.relative_to(args.directory.parent).as_posix())
