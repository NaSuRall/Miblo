#!/usr/bin/env python3
"""Test the real generated API against an isolated MySQL server. No existing DB is used."""
import argparse
import relation_checks
import evolution_checks
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

parser = argparse.ArgumentParser()
parser.add_argument('--miblo', type=Path, required=True)
source = parser.add_mutually_exclusive_group(required=True)
source.add_argument('--templates', type=Path)
source.add_argument('--remote', action='store_true', help='Download the default published release into an empty cache')
parser.add_argument('--target-dir', type=Path)
parser.add_argument('--mysql-bin-dir', type=Path)
args = parser.parse_args()
miblo = str(args.miblo.resolve())
template_args = ['--template-dir', str(args.templates.resolve())] if args.templates else []
mysql = str(args.mysql_bin_dir / 'mysql') if args.mysql_bin_dir else shutil.which('mysql')
mysqld = str(args.mysql_bin_dir / 'mysqld') if args.mysql_bin_dir else shutil.which('mysqld')
assert mysql and mysqld, 'Install MySQL (mysql and mysqld) to run this smoke test'

def free_port():
    with socket.socket() as s:
        s.bind(('127.0.0.1', 0))
        return s.getsockname()[1]

def stop(process):
    if process and process.poll() is None:
        process.terminate()
        try: process.wait(timeout=15)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)

with tempfile.TemporaryDirectory(prefix='miblo-smoke-', dir='/private/tmp' if Path('/private/tmp').is_dir() else None) as temporary:
    root = Path(temporary)
    db_port, api_port = free_port(), free_port()
    data = root / 'data'
    data.mkdir()
    db_log = root / 'mysql.log'
    api_log = root / 'api.log'
    db = api = None
    try:
        generation_env = dict(os.environ, MIBLO_CACHE_DIR=str(root / 'template-cache'))
        generation_env.pop('MIBLO_TEMPLATE_DIR', None)
        generation_env.pop('MIBLO_TEMPLATE_BASE_URL', None)
        subprocess.run([mysqld, '--no-defaults', '--initialize-insecure', f'--datadir={data}', f'--log-error={db_log}'], check=True, timeout=90)
        db = subprocess.Popen([mysqld, '--no-defaults', f'--datadir={data}', f'--port={db_port}', '--bind-address=127.0.0.1', '--mysqlx=0', f'--socket={root / "mysql.sock"}', f'--pid-file={root / "mysql.pid"}', f'--log-error={db_log}'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        db_command = [mysql, '--no-defaults', '--protocol=TCP', '-h127.0.0.1', f'-P{db_port}', '-uroot']
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if db.poll() is not None: raise RuntimeError('Temporary MySQL exited')
            probe = subprocess.run(db_command + ['-e', 'SELECT 1'], capture_output=True)
            if probe.returncode == 0: break
            time.sleep(.2)
        else: raise RuntimeError('Temporary MySQL did not become ready')
        project = root / 'api'
        subprocess.run([miblo, 'new', 'smoke-api', '--output', str(project), *template_args, '--resource', 'Product', '--field', 'title:string:unique', '--field', 'description:text?', '--field', 'price:float', '--field', 'quantity:int', '--field', 'active:bool', '--field', 'token:string:private', '--yes'], check=True, env=generation_env)
        (project / '.env').write_text(f'DATABASE_URL=mysql://root@127.0.0.1:{db_port}/miblo_smoke\n')
        subprocess.run([miblo, 'db', 'init'], cwd=project, check=True, env=generation_env)
        subprocess.run([miblo, 'db', 'check'], cwd=project, check=True, env=generation_env)
        if args.remote:
            lock = json.loads((project / '.miblo/state.json').read_text())['template']
            assert lock['local_path'] is None and lock['url'].startswith('https://github.com/NaSuRall/miblo_templates/releases/download/')
            assert lock['sha256']
            print('PASS: default GitHub release downloaded and pinned with an initially empty cache', flush=True)
        custom = project / 'src/resources/product/handlers.rs'
        custom.write_text(custom.read_text() + '\n// handwritten business rule preserved by Miblo\n')
        subprocess.run(['cargo', 'fmt'], cwd=project, check=True)
        before = custom.read_bytes()
        subprocess.run([miblo, 'add', 'resource', 'Category', '--field', 'label:string', '--yes', '--offline'], cwd=project, check=True, env=generation_env)
        assert custom.read_bytes() == before
        assert (project / 'migrations/000002_create_categories.sql').is_file()
        def cli(*arguments):
            subprocess.run([miblo, *arguments], cwd=project, check=True, env=generation_env)
        relation_checks.add_models(cli)
        subprocess.run([miblo, 'db', 'migrate'], cwd=project, check=True, env=generation_env)
        subprocess.run([miblo, 'db', 'migrate'], cwd=project, check=True, env=generation_env)
        relation_checks.inspect_schema(miblo, project, generation_env)
        cargo_args = ['--target-dir', str(args.target_dir.resolve())] if args.target_dir else []
        subprocess.run(['cargo', 'test', '--locked', *cargo_args], cwd=project, check=True, timeout=300)
        subprocess.run(['cargo', 'build', '--locked', *cargo_args], cwd=project, check=True, timeout=300)
        target = args.target_dir.resolve() if args.target_dir else project / 'target'
        env = dict(os.environ, DATABASE_URL=f'mysql://root@127.0.0.1:{db_port}/miblo_smoke', SERVER_PORT=str(api_port), SERVER_ADDRESS='127.0.0.1')
        with api_log.open('w') as log:
            api = subprocess.Popen([str(target / 'debug/smoke-api')], cwd=project, env=env, stdout=log, stderr=log)
        base = f'http://127.0.0.1:{api_port}'
        def request(method, path, payload=None, expected=200):
            body = None if payload is None else json.dumps(payload).encode()
            req = urllib.request.Request(base + path, data=body, method=method, headers={'Content-Type': 'application/json'})
            try: response = urllib.request.urlopen(req, timeout=3)
            except urllib.error.HTTPError as error: response = error
            with response:
                raw = response.read()
                assert response.status == expected, (method, path, response.status, expected, raw)
                if not raw: return None
                return json.loads(raw) if 'application/json' in response.headers.get('Content-Type', '') else raw.decode()
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if api.poll() is not None: raise RuntimeError('Generated API exited')
            try: request('GET', '/health'); break
            except (OSError, urllib.error.URLError): time.sleep(.1)
        else: raise RuntimeError('Generated API did not become ready')
        relation_checks.requests(request)
        value = dict(title='first', description='hello', price=1.5, quantity=2, active=True, token='never-return-this')
        created = request('POST', '/products', value, 201)
        resource = f'/products/{created["id"]}'
        assert 'token' not in created and created['quantity'] == 2 and created['active'] is True
        assert len(request('GET', '/products?limit=1&offset=0')) == 1
        assert request('GET', resource)['title'] == 'first'
        request('POST', '/products', value, 409)
        request('POST', '/products', dict(value, id=99), 422)
        request('POST', '/products', dict(value, title='x' * 256), 400)
        request('POST', '/products', {'title': 'missing required fields'}, 422)
        changed = request('PATCH', resource, {'description': None, 'quantity': 0, 'active': False})
        assert changed['description'] is None and changed['quantity'] == 0 and changed['active'] is False
        assert changed['title'] == 'first' and changed['price'] == 1.5 and 'token' not in changed
        request('PATCH', resource, {'title': None}, 400)
        request('PATCH', resource, {}, 400)
        request('PATCH', resource, {'id': 2}, 422)
        request('PATCH', resource, {'title': 'first'})
        request('GET', '/products?limit=101', expected=400)
        request('GET', '/products/99999', expected=404)
        request('PATCH', '/products/99999', {'title': 'missing'}, 404)
        request('POST', '/categories', {'label': 'second resource'}, 201)
        stop(api)
        evolution_checks.generate(miblo, project, generation_env, 'Product')
        cli('db', 'migrate')
        cli('db', 'migrate')
        subprocess.run(['cargo', 'build', '--locked', '--offline', *cargo_args], cwd=project, check=True, timeout=300)
        with api_log.open('a') as log:
            api = subprocess.Popen([str(target / 'debug/smoke-api')], cwd=project, env=env, stdout=log, stderr=log)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if api.poll() is not None: raise RuntimeError('Evolved API exited')
            try: request('GET', '/health'); break
            except (OSError, urllib.error.URLError): time.sleep(.1)
        else: raise RuntimeError('Evolved API did not start')
        evolution_checks.requests(request, '/products', resource, dict(value, description=None, quantity=0, active=False), layered=(project / 'src/resources/product/service.rs').exists())
        request('DELETE', resource, expected=204)
        request('DELETE', resource, expected=404)
        request('GET', resource, expected=404)
        print('PASS: creation, formatted/customized project extension, migrations, CRUD, pagination, validation, private fields, nullable PATCH, conflicts and 404s')
    except BaseException:
        for path in [db_log, api_log]:
            if path.exists(): print(path.name + ':\n' + path.read_text()[-6000:])
        raise
    finally:
        stop(api)
        stop(db)
