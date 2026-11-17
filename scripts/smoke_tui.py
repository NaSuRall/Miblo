#!/usr/bin/env python3
"""Exercise the real TUI through a PTY. Requires pyte; uses real SQLite; optional real API process."""
import argparse
import errno
import fcntl
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import pty
import select
import shlex
import shutil
import signal
import socket
import struct
import sqlite3
import subprocess
import sys
import tempfile
import termios
import time
import threading
import pyte

parser = argparse.ArgumentParser()
parser.add_argument('--miblo', type=Path, required=True)
parser.add_argument('--templates', type=Path, required=True)
parser.add_argument('--database', choices=['sqlite', 'mysql'], default='sqlite')
parser.add_argument('--capture-json', type=Path)
parser.add_argument('--api-target-dir', type=Path, help='Build and test real SQLite CRUD routes in the TUI')
args = parser.parse_args()

class Terminal:
    def __init__(self, binary, cwd, env):
        self.screen = pyte.Screen(130, 40)
        self.stream = pyte.ByteStream(self.screen)
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            os.chdir(cwd)
            os.execve(binary, [binary], env)
        self.resize(130, 40)
        self.exited = False
    def resize(self, width, height):
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.screen.resize(lines=height, columns=width)
        os.kill(self.pid, signal.SIGWINCH)
    def pump(self, timeout=.1):
        if select.select([self.fd], [], [], timeout)[0]:
            try: data = os.read(self.fd, 65536)
            except OSError as error:
                if error.errno == errno.EIO: return
                raise
            self.stream.feed(data)
    def text(self): return '\n'.join(self.screen.display)
    def wait(self, text, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.pump()
            if text in self.text(): return
        raise AssertionError(f'Missing {text!r}\n{self.text()}')
    def wait_absent(self, text, timeout=5):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.pump()
            if text not in self.text(): return
        raise AssertionError(f'Still showing {text!r}\n{self.text()}')
    def send(self, value):
        os.write(self.fd, value.encode())
    def capture(self, path):
        rows = [[dict(text=cell.data, fg=cell.fg, bg=cell.bg, bold=cell.bold) for cell in [self.screen.buffer[row][col] for col in range(self.screen.columns)]] for row in range(self.screen.lines)]
        path.write_text(json.dumps(rows, ensure_ascii=False))
    def close(self):
        if not self.exited:
            try: self.send('\x03')
            except OSError: pass
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                self.pump()
                pid, _ = os.waitpid(self.pid, os.WNOHANG)
                if pid:
                    self.exited = True
                    break
            if not self.exited:
                try: os.kill(self.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                # Closing the PTY also unblocks terminal restoration on macOS.
                os.close(self.fd)
                os.waitpid(self.pid, 0)
                return
        os.close(self.fd)

def stopped(pid):
    status = subprocess.run(['ps', '-p', str(pid), '-o', 'stat='], capture_output=True, text=True)
    return status.returncode != 0 or status.stdout.strip().startswith('Z')

class MissingRelease(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(404)
        self.end_headers()
    def log_message(self, *args):
        pass

with tempfile.TemporaryDirectory(prefix='miblo-tui-') as tmp:
    root = Path(tmp)
    # Reproduce a checkout with sibling CLI/template repositories, without any
    # environment override. A missing remote release must be recoverable in-place.
    folder = 'rust_sqlite_template_v1' if args.database == 'sqlite' else 'rust_template_v1'
    local_pack = root / 'miblo_templates' / folder
    shutil.copytree(args.templates.resolve(), local_pack)
    checkout = root / 'Miblo'
    checkout.mkdir()
    server = HTTPServer(('127.0.0.1', 0), MissingRelease)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    missing_url = f'http://127.0.0.1:{server.server_port}/missing.tar.gz'
    fake_bin = root / 'bin'; fake_bin.mkdir()
    cargo = fake_bin / 'cargo'
    cargo.write_text('#!/bin/sh\nif [ "$1" = run ]; then\n  sleep 120 &\n  echo $! > "$MIBLO_QA_PID"\n  echo "Server running on http://127.0.0.1:3000"\n  wait\nelse\n  echo "Test fixture: cargo $1 succeeded"\nfi\n')
    if args.api_target_dir:
        assert args.database == 'sqlite', 'Real API mode uses the automatic SQLite setup'
        real_cargo = shutil.which('cargo')
        command = shlex.join([real_cargo, 'run', '--locked', '--offline', '--target-dir', str(args.api_target_dir.resolve())])
        cargo.write_text('#!/bin/sh\nif [ "$1" = run ]; then\n  ' + command + ' &\n  echo $! > "$MIBLO_QA_PID"\n  wait $!\nelse\n  echo "Test fixture: cargo $1 succeeded"\nfi\n')
    cargo.chmod(0o755)
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        api_port = sock.getsockname()[1]
    editor = root / 'test editor.py'
    marker = root / 'editor-path.txt'
    editor.write_text('import pathlib, sys\npathlib.Path(sys.argv[1]).write_text(sys.argv[2])\n')
    env = dict(os.environ, TERM='xterm-256color', MIBLO_CACHE_DIR=str(root / 'cache'), PATH=str(fake_bin) + os.pathsep + os.environ['PATH'], MIBLO_QA_PID=str(root / 'api-child.pid'), VISUAL=shlex.join([sys.executable, str(editor), str(marker)]))
    for key in ['DATABASE_URL', 'SERVER_PORT', 'SERVER_ADDRESS']:
        env.pop(key, None)
    env.pop('NO_COLOR', None)
    env.pop('MIBLO_TEMPLATE_DIR', None)
    env.pop('MIBLO_TEMPLATE_BASE_URL', None)
    ui = Terminal(str(args.miblo.resolve()), checkout, env)
    try:
        ui.wait('Atelier API')
        ui.send('n'); ui.wait('Nom de la nouvelle API')
        ui.send('\x15atelier-demo\r'); ui.wait('Dossier à créer')
        ui.send('\r'); ui.wait('Port du serveur')
        ui.send('\x15' + str(api_port) + '\r'); ui.wait('Base de données')
        ui.send(('\x1b[B' if args.database == 'mysql' else '') + '\r'); ui.wait('Source du template')
        ui.wait('Dossier local — chemin proposé')
        ui.send('\x1b[B\r'); ui.wait('URL du template')
        ui.send(missing_url + '\r'); ui.wait('Créer cette API ?')
        ui.send('\r'); ui.wait('La création a échoué')
        ui.wait('HTTP 404')
        project = checkout / 'atelier-demo'
        assert not project.exists(), 'Failed template download wrote a partial project'
        ui.send('\r'); ui.wait('Source du template')
        ui.send('\r'); ui.wait('Dossier du template')
        ui.wait('miblo_templates/' + folder)
        ui.send('\r'); ui.wait('Créer cette API ?')
        ui.send('\r'); ui.wait('Configuration cohérente')
        assert (project / 'Cargo.toml').is_file()
        state = json.loads((project / '.miblo/state.json').read_text())
        assert state['template']['local_path'] == str(local_pack.resolve())
        if args.database == 'sqlite':
            ui.wait('Migrations à jour')
            assert (project / 'miblo.sqlite').is_file()
        else:
            ui.wait('Serveur MySQL')
            ui.send('\r'); ui.wait('Port MySQL')
            ui.send('\x151\r'); ui.wait('Nom de la base MySQL')
            ui.send('\x15qa_database\r'); ui.wait('Utilisateur MySQL')
            ui.send('\x15qa_user\r'); ui.wait('Mot de passe MySQL')
            ui.send('secret$:@ value'); ui.pump(.1)
            assert 'secret$:@ value' not in ui.text()
            ui.send('\r'); ui.wait('Enregistrer la connexion MySQL ?')
            ui.send('\r'); ui.wait('Connexion MySQL impossible')
            ui.wait('Aucune action en cours')
            env_text = (project / '.env').read_text()
            assert 'qa_database' in env_text and 'secret%24%3A%40%20value' in env_text
        ui.send('a'); ui.wait('Nom du modèle')
        ui.send('Product\r'); ui.wait('Nom du champ')
        ui.send('title\r'); ui.wait('Type du champ')
        ui.send('\r'); ui.wait('Options du champ')
        ui.send('\x1b[B \r'); ui.wait('Champs de la ressource')
        ui.send('\r'); ui.wait('Nom du champ')
        ui.send('quantity\r'); ui.wait('Type du champ')
        ui.send('\x1b[B\x1b[B\r'); ui.wait('Options du champ')
        ui.send(' \r'); ui.wait('Champs de la ressource')
        ui.send('\x1b[B\r'); ui.wait('Adresse de la collection')
        ui.send('\r'); ui.wait('Ajouter cette ressource ?')
        ui.send('\r'); ui.wait('1 ressource(s)')
        model = (project / 'src/resources/product/model.rs').read_text()
        migration = (project / 'migrations/000001_create_products.sql').read_text()
        sql_type = 'TEXT' if args.database == 'sqlite' else 'VARCHAR(255)'
        assert 'pub quantity: Option<i64>' in model and f'`title` {sql_type} NOT NULL UNIQUE' in migration
        if args.database == 'sqlite':
            ui.wait('Migrations à jour')
            ui.wait('Aucune action en cours')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='products'").fetchone()
            ui.send('b'); ui.wait('Tester la connexion')
            ui.send('\r'); ui.wait('connexion vérifiée'); ui.wait('Aucune action en cours')
        ui.send('\x1b[Z\r'); ui.wait('src/resources/product/handlers.rs')
        ui.send('?'); ui.wait('NAVIGATION'); ui.send('\x1b'); ui.wait_absent('NAVIGATION')
        ui.send('t'); ui.wait('Thème modifié')
        if args.capture_json: ui.capture(args.capture_json)
        ui.resize(90, 26); ui.wait_absent('Aperçu ·'); ui.wait('État BDD'); ui.wait('Réponse HTTP')
        ui.resize(130, 40); ui.wait('Aperçu ·')
        ui.send('e')
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline and not marker.exists(): ui.pump()
        assert marker.read_text().endswith('src/resources/product/handlers.rs')
        ui.wait('État')
        ui.send(':check\r'); ui.wait('Test fixture: cargo check succeeded'); ui.wait('Aucune action en cours')
        ui.send('n'); ui.wait('Nom de la nouvelle API'); ui.send('\x1b'); ui.wait_absent('Nom de la nouvelle API')
        assert not (checkout / 'my-api').exists()
        ui.send('r'); ui.wait('API en cours' if args.api_target_dir else 'API ne répond pas', timeout=120)
        first_pid = int((root / 'api-child.pid').read_text())
        if args.api_target_dir:
            ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('GET : HTTP 200')
            ui.send(':routes\rj'); ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('GET /products ·')
            ui.send(':routes\rj'); ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('Corps JSON')
            ui.send('\x15{"title":"from-tui","quantity":3}\r'); ui.wait('Envoyer cette requête ?')
            ui.send('\r'); ui.wait('Requête annulée')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == 0
            ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('Corps JSON')
            ui.send('\x15{"title":"from-tui","quantity":3}\r'); ui.wait('Envoyer cette requête ?')
            ui.send('\x1b[A\r'); ui.wait('POST : HTTP 201')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT title FROM products').fetchone()[0] == 'from-tui'
            ui.send(':routes\rj'); ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('GET /products/1 ·')
            ui.send(':routes\rj'); ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('Corps JSON')
            ui.send('\x15{"title":"updated-tui"}\r'); ui.wait('Envoyer cette requête ?')
            ui.send('\x1b[A\r'); ui.wait('PATCH : HTTP 200')
            ui.wait('updated-tui')
            if args.capture_json: ui.capture(args.capture_json)
            ui.send(':routes\rj'); ui.send('x'); ui.wait('Chemin HTTP'); ui.send('\r'); ui.wait('Envoyer cette requête ?')
            ui.send('\x1b[A\r'); ui.wait('DELETE : HTTP 204')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == 0
            ui.send('f'); ui.wait('Méthode HTTP'); ui.send('\r'); ui.wait('Chemin HTTP')
            ui.send('\x15/products/999\r'); ui.wait('GET : HTTP 404')
            ui.send('v'); ui.wait_absent('Aperçu ·'); ui.send('v'); ui.wait('Aperçu ·')

        ui.send('s'); ui.wait('API arrêtée')
        assert stopped(first_pid), f'API child {first_pid} survived stop'
        if args.database == 'sqlite':
            (project / 'migrations/000002_extra.sql').write_text('CREATE TABLE extra (id INTEGER PRIMARY KEY);')
            ui.wait('Mise à jour BDD requise')
            # Reopening must detect pending migrations from the database, not cached UI state.
            ui.send(':open ' + str(project) + '\r'); ui.wait('Mise à jour BDD requise')
            ui.send('b'); ui.wait('Appliquer les migrations'); ui.send('\x1b[B\r'); ui.wait('Confirmer')
            ui.send('\r'); ui.wait('Migrations à jour'); ui.wait('Aucune action en cours')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute("SELECT name FROM sqlite_master WHERE name='extra'").fetchone()

        ui.send('r'); ui.wait('API en cours' if args.api_target_dir else 'API ne répond pas', timeout=120)
        second_pid = int((root / 'api-child.pid').read_text())
        assert second_pid != first_pid
        ui.send('q')
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            ui.pump()
            pid, status = os.waitpid(ui.pid, os.WNOHANG)
            if pid:
                ui.exited = True
                assert os.waitstatus_to_exitcode(status) == 0
                break
        else: raise AssertionError('TUI did not exit')
        assert stopped(second_pid), f'API child {second_pid} survived quit'
        print('PASS: local template discovery, HTTP 404 recovery without losing the draft, TUI creation, arrow selections, field options, resource generation, preview, themes, resize, editor handoff, commands, cancellation, HTTP-based status, pending migrations, code toggle, optional real CRUD requests, process stop and quit cleanup')
    except BaseException:
        print(ui.text())
        raise
    finally:
        ui.close()
        server.shutdown()
        server.server_close()
        server_thread.join()
