#!/usr/bin/env python3
"""Generate and run a real SQLite API, including empty-project startup and CRUD."""
import argparse
import relation_checks
import evolution_checks
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

parser = argparse.ArgumentParser()
parser.add_argument('--miblo', type=Path, required=True)
source = parser.add_mutually_exclusive_group(required=True)
source.add_argument('--templates', type=Path)
source.add_argument('--remote', action='store_true')
parser.add_argument('--target-dir', type=Path, required=True)
parser.add_argument('--legacy-templates', type=Path, help='Start with an old local pack, then update before adding the remaining models')
args = parser.parse_args()
miblo = str(args.miblo.resolve())
target = args.target_dir.resolve()
template_args = ['--template-dir', str(args.templates.resolve())] if args.templates else []
if args.legacy_templates:
    assert args.templates, '--legacy-templates requires --templates for the new pack'
    template_args = ['--template-dir', str(args.legacy_templates.resolve())]

def stop(process):
    if process and process.poll() is None:
        process.terminate()
        try: process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

with tempfile.TemporaryDirectory(prefix='miblo-sqlite-', dir='/private/tmp' if Path('/private/tmp').exists() else None) as tmp:
    root = Path(tmp)
    project = root / 'api'
    env = dict(os.environ, MIBLO_CACHE_DIR=str(root / 'cache'))
    for key in ['MIBLO_TEMPLATE_DIR', 'MIBLO_TEMPLATE_BASE_URL', 'DATABASE_URL']:
        env.pop(key, None)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    env.update(SERVER_ADDRESS='127.0.0.1', SERVER_PORT=str(port))
    log_path = root / 'api.log'
    api = None
    def cli(*arguments):
        subprocess.run([miblo, *arguments], cwd=project if project.exists() else root, env=env, check=True)
    def cargo(action):
        subprocess.run(['cargo', action, '--locked', '--offline', '--target-dir', str(target)], cwd=project, env=env, check=True, timeout=300)
    def request(method, path, value=None, expected=200):
        body = None if value is None else json.dumps(value).encode()
        req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', data=body, method=method, headers={'Content-Type':'application/json'})
        try: response = urllib.request.urlopen(req, timeout=15)
        except urllib.error.HTTPError as error: response = error
        with response:
            raw = response.read()
            assert response.status == expected, (method, path, response.status, expected, raw)
            return json.loads(raw) if raw and 'application/json' in response.headers.get('Content-Type','') else raw
    def start():
        with log_path.open('a') as log:
            process = subprocess.Popen([str(target / 'debug/sqlite-smoke')], cwd=project, env=env, stdout=log, stderr=log)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if process.poll() is not None: raise AssertionError('API exited: ' + log_path.read_text())
            try:
                request('GET', '/health')
                return process
            except (OSError, urllib.error.URLError): time.sleep(.1)
        stop(process)
        raise AssertionError('API did not start')
    try:
        cli('new', 'sqlite-smoke', '--output', str(project), '--database', 'sqlite', *template_args, '--yes')
        assert (project / '.env').exists() and not (project / 'compose.yaml').exists()
        if args.remote:
            lock = json.loads((project / '.miblo/state.json').read_text())['template']
            assert lock['local_path'] is None and lock['url'].endswith('/v0.4.0/rust-axum-sqlite.tar.gz')
        cargo('build')
        assert not (project / 'miblo.sqlite').exists()
        api = start()
        assert (project / 'miblo.sqlite').is_file(), 'API startup must create SQLite automatically'
        stop(api)
        cli('add', 'User', 'email:string:unique', 'name:string', 'note:text?', 'score:float', 'active:bool', 'token:string:private', '--yes', '--offline')
        custom = project / 'src/resources/user/handlers.rs'
        custom.write_text(custom.read_text() + '\n// preserved handwritten code\n')
        before = custom.read_bytes()
        if args.legacy_templates:
            cli('template', 'update', '--template-dir', str(args.templates.resolve()))
            assert custom.read_bytes() == before
        cli('add', 'Category', 'label:string', '--yes', '--offline')
        assert custom.read_bytes() == before
        if args.legacy_templates:
            assert not (project / 'src/resources/user/service.rs').exists()
            assert (project / 'src/resources/category/service.rs').exists()
        relation_checks.add_models(cli)
        cli('db', 'migrate')
        cli('db', 'check')
        with sqlite3.connect(project / 'miblo.sqlite') as connection:
            assert connection.execute("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('users','categories')").fetchone()[0] == 2
        relation_checks.inspect_schema(miblo, project, env)
        cargo('test')
        cargo('build')
        api = start()
        relation_checks.requests(request)
        value = dict(email='first@example.test', name='First', note='hello', score=1.5, active=True, token='private-example')
        created = request('POST', '/users', value, 201)
        assert 'token' not in created and created['active'] is True
        path = f'/users/{created["id"]}'
        assert request('GET', path)['email'] == value['email']
        request('POST', '/users', value, 409)
        request('POST', '/users', dict(value, id=12), 422)
        request('POST', '/users', dict(value, name='x'*256), 400)
        changed = request('PATCH', path, dict(note=None, active=False, score=0.0))
        assert changed['note'] is None and changed['active'] is False and changed['score'] == 0
        request('PATCH', path, {'name':None}, 400)
        request('PATCH', path, {}, 400)
        request('PATCH', path, {'name':'First'})
        with ThreadPoolExecutor(max_workers=4) as pool:
            list(pool.map(lambda i: request('PATCH', path, {'score':float(i)}), range(8)))
        assert len(request('GET', '/users?limit=1&offset=0')) == 1
        request('GET', '/users?limit=101', expected=400)
        request('GET', '/users?offset=18446744073709551615', expected=400)
        request('PATCH', '/users/999999', {'name':'missing'}, 404)
        request('GET', '/users/999999', expected=404)
        request('POST', '/categories', {'label':'Second table'}, 201)
        stop(api)
        api = start()
        assert request('GET', path)['email'] == value['email'], 'SQLite data must persist across restarts'
        stop(api)
        with sqlite3.connect(project / 'miblo.sqlite') as connection:
            connection.execute('PRAGMA foreign_keys=ON')
            connection.execute('ALTER TABLE users ADD COLUMN manual_column TEXT')
            connection.execute('UPDATE users SET manual_column = ?', ('preserved',))
            connection.execute('CREATE INDEX manual_index ON users(name)')
            connection.execute('CREATE TABLE manual_child (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id) ON DELETE CASCADE)')
            connection.execute('INSERT INTO manual_child VALUES (1, ?)', (created['id'],))
            connection.execute('CREATE TABLE audit (id INTEGER)')
            connection.execute('CREATE TRIGGER manual_trigger AFTER UPDATE ON users BEGIN INSERT INTO audit VALUES (new.id); END')
        evolution_checks.generate(miblo, project, env, 'User')
        with sqlite3.connect(project / 'miblo.sqlite') as connection:
            assert 'alias' not in [r[1] for r in connection.execute('PRAGMA table_info(users)')], 'Generation applied migrations'
        cargo('build')
        api = start()  # Stock generated SQLx startup must apply additive migrations too.
        with sqlite3.connect(project / 'miblo.sqlite') as connection:
            assert connection.execute('SELECT manual_column FROM users').fetchone()[0] == 'preserved'
            assert connection.execute('SELECT user_id FROM manual_child').fetchone()[0] == created['id']
            assert connection.execute('SELECT COUNT(*) FROM audit').fetchone()[0] == 0, 'Migration unexpectedly updated existing rows'
            assert connection.execute("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('manual_index','manual_trigger')").fetchone()[0] == 2
            assert not connection.execute('PRAGMA foreign_key_check').fetchall()
        evolution_checks.requests(request, '/users', path, dict(value, note=None, active=False, score=request('GET', path)['score']), layered=(project / 'src/resources/user/service.rs').exists())
        with sqlite3.connect(project / 'miblo.sqlite') as connection:
            assert connection.execute('SELECT COUNT(*) FROM audit').fetchone()[0] > 0, 'Custom trigger no longer works'
        request('DELETE', path, expected=204)
        request('DELETE', path, expected=404)
        print('PASS: automatic SQLite setup, empty startup, models/tables, migrations, customized code preservation, CRUD, concurrent PATCH, unique/private/nullable fields and restart persistence')
    finally:
        stop(api)
