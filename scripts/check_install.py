#!/usr/bin/env python3
"""Build the distributable crate and install it outside the checkout, without publishing.

Requires Python 3.12+, Rust and rustfmt. Templates are downloaded from the default
public release; --cargo-offline only disables Cargo's dependency downloads.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib


def run(arguments, *, cwd, env, capture=False):
    print('+', ' '.join(map(str, arguments)), flush=True)
    return subprocess.run(
        list(map(str, arguments)), cwd=cwd, env=env, check=True,
        text=True, capture_output=capture, timeout=1200,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target-dir', type=Path, default=Path('target/beta-check'))
    parser.add_argument('--allow-dirty', action='store_true', help='Local development only')
    parser.add_argument('--cargo-offline', action='store_true')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    target = args.target_dir.resolve()
    package = tomllib.loads((repo / 'Cargo.toml').read_text(encoding='utf-8'))['package']
    name, version = package['name'], package['version']
    prefix = f'{name}-{version}'
    env = dict(os.environ)
    for key in ['MIBLO_TEMPLATE_DIR', 'MIBLO_TEMPLATE_BASE_URL', 'MIBLO_CACHE_DIR',
                'DATABASE_URL', 'SERVER_ADDRESS', 'SERVER_PORT', 'CARGO_TARGET_DIR']:
        env.pop(key, None)
    cargo_options = ['--locked'] + (['--offline'] if args.cargo_offline else [])
    # The release-mode installation below verifies the extracted archive itself.
    run(['cargo', 'package', '--no-verify', *cargo_options, '--target-dir', target,
         *(['--allow-dirty'] if args.allow_dirty else [])], cwd=repo, env=env)
    archive = target / 'package' / f'{prefix}.crate'
    with tempfile.TemporaryDirectory(prefix='miblo install ') as temporary:
        root = Path(temporary).resolve()
        with tarfile.open(archive, 'r:gz') as crate:
            members = crate.getmembers()
            names = set()
            for member in members:
                parts = Path(member.name).parts
                if not parts or parts[0] != prefix or '..' in parts or not (member.isfile() or member.isdir()):
                    raise AssertionError(f'Unexpected archive entry: {member.name}')
                relative = '/'.join(parts[1:])
                names.add(relative)
                if relative and not (relative.startswith(('src/', 'tests/')) or relative in {
                    'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', 'README.md',
                    'LICENSE', 'CHANGELOG.md', '.cargo_vcs_info.json',
                }):
                    raise AssertionError(f'Unwanted file in crate: {relative}')
            required = {'Cargo.toml', 'Cargo.lock', 'src/main.rs', 'LICENSE', 'README.md', 'CHANGELOG.md'}
            if not required <= names:
                raise AssertionError(f'Missing package files: {required - names}')
            crate.extractall(root / 'source', filter='data')
        source = root / 'source' / prefix
        install = root / 'installation'
        work = root / 'workspace'
        work.mkdir()
        env['MIBLO_CACHE_DIR'] = str(root / 'empty template cache')
        run(['cargo', 'install', '--path', source, '--root', install,
             '--target-dir', target / 'install', *cargo_options], cwd=work, env=env)
        binary = install / 'bin' / ('miblo.exe' if os.name == 'nt' else 'miblo')
        result = run([binary, '--version'], cwd=work, env=env, capture=True)
        if result.stdout.strip() != f'miblo {version}':
            raise AssertionError(f'Unexpected installed version: {result.stdout}')
        run([binary, '--help'], cwd=work, env=env, capture=True)
        project = work / 'my api'
        run([binary, 'new', 'install-smoke', '--database', 'sqlite', '--output', project,
             '--resource', 'User', '--field', 'name:string', '--yes'], cwd=work, env=env)
        state = json.loads((project / '.miblo/state.json').read_text(encoding='utf-8'))
        pin = state['template']
        if pin['local_path'] is not None or not pin['url'].startswith(
            'https://github.com/NaSuRall/miblo_templates/releases/download/'
        ) or not pin['sha256']:
            raise AssertionError('Installation did not use a pinned public template')
        run([binary, 'add', 'Group', 'title:string', '--yes', '--offline'], cwd=project, env=env)
        run([binary, 'edit', 'User', 'group_id:int?:ref=Group', '--yes', '--offline'], cwd=project, env=env)
        run([binary, 'route', 'add', 'members', '--resource', 'User', '--action', 'create',
             '--path', '/members', '--yes', '--offline'], cwd=project, env=env)
        run([binary, 'db', 'migrate'], cwd=project, env=env)
        run([binary, 'doctor'], cwd=project, env=env)
        run(['cargo', 'check', *cargo_options, '--target-dir', target / 'generated'], cwd=project, env=env)
        print(f'PASS: {name} {version} installed from {archive}; public templates, offline cache, models, relation, route and generated API compile', flush=True)


if __name__ == '__main__':
    main()
