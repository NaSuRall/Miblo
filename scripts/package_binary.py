#!/usr/bin/env python3
"""Package a native release binary with its license, README and SHA-256."""
import argparse
import hashlib
import re
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True, choices=[
        'x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc',
        'aarch64-apple-darwin', 'x86_64-apple-darwin',
    ])
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?', version):
        raise ValueError('Invalid release version')
    output_dir = root / 'dist'
    suffix = '.exe' if 'windows' in args.target else ''
    binary = root / 'target' / args.target / 'release' / ('miblo' + suffix)
    result = subprocess.run([str(binary), '--version'], check=True, capture_output=True, text=True)
    assert result.stdout.strip() == f'miblo {version}', result.stdout
    subprocess.run([str(binary), '--help'], check=True, stdout=subprocess.DEVNULL)
    output_dir.mkdir(parents=True, exist_ok=True)
    stem = f'miblo-{version}-{args.target}'
    with tempfile.TemporaryDirectory(prefix='miblo-release-') as temp:
        bundle = Path(temp) / stem
        bundle.mkdir()
        for source in (binary, root / 'README.md', root / 'LICENSE', root / 'CHANGELOG.md'):
            shutil.copy2(source, bundle / source.name)
        if suffix:
            archive = output_dir / f'{stem}.zip'
            with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as output:
                for file in sorted(bundle.iterdir()):
                    output.write(file, f'{stem}/{file.name}')
        else:
            archive = output_dir / f'{stem}.tar.gz'
            with tarfile.open(archive, 'w:gz') as output:
                output.add(bundle, arcname=stem)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + '.sha256').write_text(f'{digest}  {archive.name}\n')
    print(archive)


if __name__ == '__main__':
    main()
