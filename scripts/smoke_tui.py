#!/usr/bin/env python3
"""Exercise the real TUI through a PTY. Requires pyte; uses real SQLite; optional real API process."""
import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import threading
import urllib.request
from tui_terminal import Terminal

parser = argparse.ArgumentParser()
parser.add_argument('--miblo', type=Path, required=True)
parser.add_argument('--templates', type=Path, required=True)
parser.add_argument('--language', choices=['en', 'fr'], default='en')
parser.add_argument('--database', choices=['sqlite', 'mysql'], default='sqlite')
parser.add_argument('--capture-json', type=Path)
parser.add_argument('--splash-capture-json', type=Path)
parser.add_argument('--request-capture-json', type=Path)
parser.add_argument('--schema-capture-json', type=Path)
parser.add_argument('--edit-capture-json', type=Path)
parser.add_argument('--api-target-dir', type=Path, help='Build and test real SQLite CRUD routes in the TUI')
parser.add_argument('--cargo-offline', action='store_true', help='Use only cached Rust dependencies')
args = parser.parse_args()

def label(english, french):
    return french if args.language == 'fr' else english


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
    folder = ('rust_sqlite_template_v' if args.database == 'sqlite' else 'rust_template_v') + str(json.loads((args.templates / 'manifest.json').read_text())['schema_version'])
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
    env = dict(os.environ, MIBLO_LANGUAGE=args.language, MIBLO_CONFIG_DIR=str(root / 'preferences'), TERM='xterm-256color', MIBLO_CACHE_DIR=str(root / 'cache'), PATH=str(fake_bin) + os.pathsep + os.environ['PATH'], MIBLO_QA_PID=str(root / 'api-child.pid'), VISUAL=shlex.join([sys.executable, str(editor), str(marker)]))
    for key in ['DATABASE_URL', 'SERVER_PORT', 'SERVER_ADDRESS']:
        env.pop(key, None)
    env.pop('NO_COLOR', None)
    env.pop('MIBLO_TEMPLATE_DIR', None)
    env.pop('MIBLO_TEMPLATE_BASE_URL', None)
    ui = Terminal(str(args.miblo.resolve()), checkout, env)
    try:
        ui.wait(label('Your API takes shape.', 'Votre API prend forme.'))
        ui.wait('  '.join(['█     █', '█████', '████ ', '█    ', ' ███ ']).rstrip())
        assert any(cell.fg == '4696ff' for row in ui.screen.buffer.values() for cell in row.values()), 'Logo must be blue'
        if args.splash_capture_json: ui.capture(args.splash_capture_json)
        ui.wait(label('API workspace', 'Atelier API'))
        ui.send('n'); ui.wait(label('New API name', 'Nom de la nouvelle API'))
        ui.send('\x15atelier-demo\r'); ui.wait(label('Directory to create', 'Dossier à créer'))
        ui.send('\r'); ui.wait(label('Server port', 'Port du serveur'))
        ui.send('\x15' + str(api_port) + '\r'); ui.wait(label('Database', 'Base de données'))
        ui.send(('\x1b[B' if args.database == 'mysql' else '') + '\r'); ui.wait(label('Template source', 'Source du template'))
        ui.wait(label('Local directory — suggested path', 'Dossier local — chemin proposé'))
        ui.send('\x1b[B\x1b[B\r'); ui.wait(label('Template .tar.gz URL', 'URL du template'))
        ui.send(missing_url + '\r'); ui.wait(label('Create this API?', 'Créer cette API ?'))
        ui.send('\r'); ui.wait(label('Creation failed', 'La création a échoué'))
        ui.wait('HTTP 404')
        project = checkout / 'atelier-demo'
        assert not project.exists(), 'Failed template download wrote a partial project'
        ui.send('\r'); ui.wait(label('Template source', 'Source du template'))
        ui.send('\x1b[B\r'); ui.wait(label('Template directory', 'Dossier du template'))
        ui.wait('miblo_templates/' + folder)
        ui.send('\r'); ui.wait(label('Create this API?', 'Créer cette API ?'))
        ui.send('\r'); ui.wait(label('Configuration consistent', 'Configuration cohérente'))
        assert (project / 'Cargo.toml').is_file()
        state = json.loads((project / '.miblo/state.json').read_text())
        assert state['template']['local_path'] == str(local_pack.resolve())
        if args.database == 'sqlite':
            ui.wait(label('Migrations up to date', 'Migrations à jour'))
            assert (project / 'miblo.sqlite').is_file()
        else:
            ui.wait(label('MySQL server', 'Serveur MySQL'))
            ui.send('\r'); ui.wait(label('MySQL port', 'Port MySQL'))
            ui.send('\x151\r'); ui.wait(label('MySQL database name', 'Nom de la base MySQL'))
            ui.send('\x15qa_database\r'); ui.wait(label('MySQL user', 'Utilisateur MySQL'))
            ui.send('\x15qa_user\r'); ui.wait(label('MySQL password', 'Mot de passe MySQL'))
            ui.send('secret$:@ value'); ui.pump(.1)
            assert 'secret$:@ value' not in ui.text()
            ui.send('\r'); ui.wait(label('Save the MySQL connection?', 'Enregistrer la connexion MySQL ?'))
            ui.send('\r'); ui.wait(label('Cannot connect to MySQL', 'Connexion MySQL impossible'))
            ui.wait(label('No action running', 'Aucune action en cours'))
            env_text = (project / '.env').read_text()
            assert 'qa_database' in env_text and 'secret%24%3A%40%20value' in env_text
        ui.send('a'); ui.wait(label('Model name', 'Nom du modèle'))
        ui.send('Product\r'); ui.wait(label('Field name', 'Nom du champ'))
        ui.send('title\r'); ui.wait(label('Field type', 'Type du champ'))
        ui.send('\r'); ui.wait(label('Field options', 'Options du champ'))
        ui.send('\x1b[B \r'); ui.wait(label('Model fields', 'Champs de la ressource'))
        ui.send('\r'); ui.wait(label('Field name', 'Nom du champ'))
        ui.send('quantity\r'); ui.wait(label('Field type', 'Type du champ'))
        ui.send('\x1b[B\x1b[B\r'); ui.wait(label('Field options', 'Options du champ'))
        ui.send(' \r'); ui.wait(label('Model fields', 'Champs de la ressource'))
        ui.send('\x1b[B\r'); ui.wait(label('Collection path', 'Adresse de la collection'))
        ui.send('\r'); ui.wait(label('Add this model?', 'Ajouter cette ressource ?'))
        ui.send('\r'); ui.wait(label('1 model(s)', '1 ressource(s)'))
        model = (project / 'src/resources/product/model.rs').read_text()
        migration = (project / 'migrations/000001_create_products.sql').read_text()
        sql_type = 'TEXT' if args.database == 'sqlite' else 'VARCHAR(255)'
        assert 'pub quantity: Option<i64>' in model and f'`title` {sql_type} NOT NULL UNIQUE' in migration
        if args.database == 'sqlite':
            ui.wait(label('Migrations up to date', 'Migrations à jour'))
            ui.wait(label('No action running', 'Aucune action en cours'))
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='products'").fetchone()
            ui.send('b'); ui.wait(label('Test connection', 'Tester la connexion'))
            ui.send('\r'); ui.wait(label('connection verified', 'connexion vérifiée')); ui.wait(label('No action running', 'Aucune action en cours'))
        if args.database == 'sqlite':
            ui.send('a'); ui.wait(label('Model name', 'Nom du modèle')); ui.send('Stock\r'); ui.wait(label('Field name', 'Nom du champ'))
            ui.send('location\r'); ui.wait(label('Field type', 'Type du champ')); ui.send('\r'); ui.wait(label('Field options', 'Options du champ'))
            ui.send('\r'); ui.wait(label('Model fields', 'Champs de la ressource'))
            ui.send('\x1b[B\x1b[B\x1b[B\r'); ui.wait(label('Model / table to reference', 'Modèle / table à référencer'))
            ui.send('\r'); ui.wait(label('Foreign key name', 'Nom de la clé étrangère')); ui.wait('product_id')
            ui.send('\r'); ui.wait(label('Field options', 'Options du champ')); ui.send(' \r'); ui.wait(label('When the referenced row', 'Si la ligne référencée'))
            ui.send('\x1b[B\x1b[B\r'); ui.wait(label('Model fields', 'Champs de la ressource'))
            ui.send('\x1b[B\r'); ui.wait(label('Collection path', 'Adresse de la collection')); ui.send('\r'); ui.wait(label('Add this model?', 'Ajouter cette ressource ?'))
            ui.wait('SET NULL'); ui.send('\r'); ui.wait(label('2 model(s)', '2 ressource(s)')); ui.wait(label('Migrations up to date', 'Migrations à jour')); ui.wait(label('No action running', 'Aucune action en cours'))
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                keys = connection.execute("PRAGMA foreign_key_list('stocks')").fetchall()
                assert keys[0][2:5] == ('products', 'product_id', 'id') and keys[0][6] == 'SET NULL'
            ui.send('d'); ui.wait(label('Live schema', 'Schéma réel')); ui.wait('Tables (2)'); ui.send('j')
            ui.wait('TABLE stocks'); ui.wait('product_id → products.id'); ui.wait('SET NULL')
            if args.schema_capture_json: ui.capture(args.schema_capture_json)
            ui.send('p'); ui.wait(label('Declared schema', 'Schéma déclaré')); ui.send('j'); ui.wait('TABLE stocks')
            ui.send('d'); ui.wait(label('HTTP response', 'Réponse HTTP'))
            ui.send('u'); ui.wait(label('Templates for new models', 'Templates pour les futurs modèles')); ui.send('\x1b[B\r'); ui.wait(label('New template directory', 'Dossier du nouveau template'))
            ui.send('\r'); ui.wait(label('Use this template', 'Utiliser ce template')); ui.send('\r'); ui.wait('Templates : exit status: 0'); ui.wait(label('No action running', 'Aucune action en cours'))
        ui.send(':routes\r\x1b[Z\x1b[Z\r'); ui.wait('src/resources/product/handlers.rs')
        ui.send('?'); ui.wait('NAVIGATION'); ui.send('\x1b'); ui.wait_absent('NAVIGATION')
        ui.send('t'); ui.wait(label('Theme changed', 'Thème modifié'))
        if args.capture_json: ui.capture(args.capture_json)
        ui.resize(90, 26); ui.wait_absent(label('Preview ·', 'Aperçu ·')); ui.wait(label('DB status', 'État BDD')); ui.wait(label('HTTP response', 'Réponse HTTP'))
        ui.resize(130, 40); ui.wait(label('Preview ·', 'Aperçu ·'))
        # Resizing hides the code panel and moves focus to Routes; select the file again.
        ui.send(':routes\r\x1b[Z\x1b[Z\r'); ui.wait('src/resources/product/handlers.rs')
        ui.send('e')
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline and not marker.exists(): ui.pump()
        assert marker.read_text().endswith('src/resources/product/handlers.rs')
        ui.wait(label('status', 'État'))
        ui.send(':check\r'); ui.wait('Test fixture: cargo check succeeded'); ui.wait(label('No action running', 'Aucune action en cours'))
        ui.send('n'); ui.wait(label('New API name', 'Nom de la nouvelle API')); ui.send('\x1b'); ui.wait_absent(label('New API name', 'Nom de la nouvelle API'))
        assert not (checkout / 'my-api').exists()
        if args.api_target_dir:
            # Resolve/build once outside the UI timeout on a fresh CI runner.
            subprocess.run([real_cargo, 'build', '--locked', *(['--offline'] if args.cargo_offline else []),
                            '--target-dir', str(args.api_target_dir.resolve())],
                           cwd=project, env=env, check=True, timeout=600)
        ui.send('r'); ui.wait(label('API running', 'API en cours') if args.api_target_dir else label('API not responding', 'API ne répond pas'), timeout=120)
        first_pid = int((root / 'api-child.pid').read_text())
        # Switch with a running API. Subsequent generation jobs must use the new
        # language even though the parent environment still holds the old one.
        tracked = [project / 'miblo.yaml', project / '.miblo/state.json',
                   *project.glob('src/**/*.rs'), *project.glob('migrations/*.sql')]
        before_language = {path: path.read_bytes() for path in tracked}
        ui.send('L'); ui.wait(label('Applies immediately', 'Appliqué immédiatement'))
        ui.send(('\x1b[B' if args.language == 'en' else '\x1b[A') + '\r')
        args.language = 'fr' if args.language == 'en' else 'en'
        ui.wait(label('Language saved: English', 'Langue enregistrée : français'))
        ui.wait(label('API running', 'API en cours') if args.api_target_dir else label('API not responding', 'API ne répond pas'))
        assert not stopped(first_pid), 'Changing language stopped the API'
        assert {path: path.read_bytes() for path in tracked} == before_language, 'Changing language modified project files'
        assert json.loads((root / 'preferences/settings.json').read_text())['language'] == args.language
        if args.api_target_dir:
            ui.send('x'); ui.wait(label('HTTP path', 'Chemin HTTP')); ui.send('\r'); ui.wait('GET : HTTP 200')
            ui.send(':routes\rj'); ui.send('x'); ui.wait(label('HTTP path', 'Chemin HTTP')); ui.send('\r'); ui.wait('GET /products ·')
            ui.send(':routes\rj'); ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête'))
            ui.wait(label('title · text', 'title · texte')); ui.wait(label('quantity · integer', 'quantity · entier')); ui.wait(label('Send request', 'Envoyer la requête'))
            ui.send('\x1b'); ui.wait(label('Request cancelled', 'Requête annulée'))
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == 0
            # Send the preconstructed POST without entering any JSON or field values.
            ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête'))
            ui.wait(label('quantity · integer', 'quantity · entier')); ui.wait(label('Fields prefilled', 'Champs préremplis')); ui.pump(.1)
            if args.request_capture_json: ui.capture(args.request_capture_json)
            ui.resize(75, 22)
            for _ in range(4): ui.pump(.1)
            ui.wait(label('Send request', 'Envoyer la requête')); ui.wait('quantity')
            ui.resize(130, 40)
            for _ in range(4): ui.pump(.1)
            ui.send('\r'); ui.wait('POST : HTTP 201')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                created_row = connection.execute('SELECT title, quantity FROM products').fetchone()
                assert created_row[0].startswith(label('example-', 'exemple-')) and created_row[1] is None
            # Another POST receives a different unique title automatically.
            ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête')); ui.send('\r'); ui.wait('POST : HTTP 201')
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                ui.pump()
                with sqlite3.connect(project / 'miblo.sqlite') as connection:
                    if connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == 2: break
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT COUNT(DISTINCT title) FROM products').fetchone()[0] == 2
            ui.send(':routes\rj'); ui.send('x'); ui.wait(label('HTTP path', 'Chemin HTTP')); ui.send('\r'); ui.wait('GET /products/1 ·')
            ui.send(':routes\rj'); ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête'))
            ui.wait(label('not sent', 'non envoyé')); ui.send('jj\r'); ui.wait(label('Enter only the value', 'Saisis uniquement la valeur'))
            ui.send('\x15updated-tui\r'); ui.wait(label('Send request', 'Envoyer la requête'))
            ui.send('kk\r'); ui.wait('PATCH : HTTP 200'); ui.wait('updated-tui')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT title,quantity FROM products WHERE id=1').fetchone() == ('updated-tui', None)
            if args.capture_json: ui.capture(args.capture_json)
            ui.send(':routes\rj'); ui.send('x'); ui.wait(label('HTTP path', 'Chemin HTTP')); ui.send('\r'); ui.wait(label('Send this request?', 'Envoyer cette requête ?'))
            ui.send('\x1b[A\r'); ui.wait('DELETE : HTTP 204')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == 1
            ui.send('f'); ui.wait(label('HTTP method', 'Méthode HTTP')); ui.send('\r'); ui.wait(label('HTTP path', 'Chemin HTTP'))
            ui.send('\x15/products/999\r'); ui.wait('GET : HTTP 404')
            ui.send('v'); ui.wait_absent(label('Preview ·', 'Aperçu ·')); ui.send('v'); ui.wait(label('Preview ·', 'Aperçu ·'))

        if args.api_target_dir:
            ui.send(':routes\rjj'); ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête'))
            ui.send('jjj\r'); ui.wait(label('first available rows', 'premières lignes disponibles'))
            ui.wait('#2'); ui.send('\r'); ui.send('kkk\r'); ui.wait('POST /stocks ·'); ui.wait('POST : HTTP 201')
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT product_id FROM stocks').fetchone()[0] == 2
        ui.send('s'); ui.wait(label('API stopped', 'API arrêtée'))
        assert stopped(first_pid), f'API child {first_pid} survived stop'
        if args.database == 'sqlite':
            (project / 'migrations/000003_extra.sql').write_text('CREATE TABLE extra (id INTEGER PRIMARY KEY);')
            ui.wait(label('Database update required', 'Mise à jour BDD requise'))
            # Reopening must detect pending migrations from the database, not cached UI state.
            ui.send(':open ' + str(project) + '\r'); ui.wait(label('Database update required', 'Mise à jour BDD requise'))
            ui.send('b'); ui.wait(label('Apply migrations', 'Appliquer les migrations')); ui.send('\x1b[B\r'); ui.wait(label('Confirm', 'Confirmer'))
            ui.send('\r'); ui.wait(label('Migrations up to date', 'Migrations à jour')); ui.wait(label('No action running', 'Aucune action en cours'))
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute("SELECT name FROM sqlite_master WHERE name='extra'").fetchone()

            # Edit a populated model through arrows, with a relation to a later model.
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                old_id = connection.execute('INSERT INTO products(title,quantity) VALUES (?,?)', ('before-edit', 9)).lastrowid
                stock_id = connection.execute('INSERT INTO stocks(location,product_id) VALUES (?,?)', ('preserved-stock', old_id)).lastrowid
            handler = project / 'src/resources/product/handlers.rs'
            handler.write_text(handler.read_text() + '\n// custom logic preserved by UI editing\n')
            migrations = {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')}
            ui.send('m'); ui.wait(label('Edit an existing model', 'Modifier un modèle existant')); ui.send('\r'); ui.wait(label('existing fields', 'champs existants'))
            ui.wait('title:string:unique'); ui.send('\x1b'); ui.wait_absent(label('existing fields', 'champs existants'))
            assert {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')} == migrations
            ui.send('m'); ui.wait(label('Edit an existing model', 'Modifier un modèle existant')); ui.send('\r'); ui.wait(label('existing fields', 'champs existants'))
            ui.send('\r'); ui.wait(label('New optional field name', 'Nom du nouveau champ')); ui.send('alias\r'); ui.wait(label('Field type', 'Type du champ'))
            ui.send('\r'); ui.wait(label('Field options', 'Options du champ')); ui.wait(label('existing rows keep NULL', 'anciennes lignes à NULL'))
            ui.send(' '); ui.pump(.1); ui.wait(label('[×] Optional', '[×] Facultatif'))
            ui.send('\x1b[B \r'); ui.wait(label('Model fields', 'Champs de la ressource'))
            ui.send('\x1b[B\x1b[B\x1b[B\r'); ui.wait(label('Model / table to reference', 'Modèle / table à référencer'))
            ui.send('\x1b[B\r'); ui.wait(label('Foreign key name', 'Nom de la clé étrangère')); ui.wait('stock_id')
            ui.send('\r'); ui.wait(label('Field options', 'Options du champ')); ui.send('\r'); ui.wait(label('When the referenced row', 'Si la ligne référencée'))
            ui.send('\x1b[B\x1b[B\r'); ui.wait(label('Model fields', 'Champs de la ressource'))
            ui.send('\x1b[B\r'); ui.wait(label('Generate the model update', 'Générer la modification du modèle')); ui.wait('ALTER TABLE')
            if args.edit_capture_json: ui.capture(args.edit_capture_json)
            ui.send('\r'); ui.wait(label('Update model : exit status: 0', 'Modification du modèle : exit status: 0')); ui.wait(label('No action running', 'Aucune action en cours'))
            ui.wait(label('Database update required', 'Mise à jour BDD requise'))
            assert 'custom logic preserved by UI editing' in handler.read_text()
            for path, content in migrations.items(): assert path.read_bytes() == content
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert 'alias' not in [r[1] for r in connection.execute('PRAGMA table_info(products)')]
            ui.send('b'); ui.wait(label('Apply migrations', 'Appliquer les migrations')); ui.send('\x1b[B\r'); ui.wait(label('Confirm', 'Confirmer'))
            ui.send('\r'); ui.wait(label('Migrations up to date', 'Migrations à jour')); ui.wait(label('No action running', 'Aucune action en cours'))
            with sqlite3.connect(project / 'miblo.sqlite') as connection:
                assert connection.execute('SELECT title,quantity,alias,stock_id FROM products WHERE id=?', (old_id,)).fetchone() == ('before-edit', 9, None, None)
                assert connection.execute('SELECT product_id FROM stocks WHERE id=?', (stock_id,)).fetchone()[0] == old_id
            ui.send('d'); ui.wait(label('Live schema', 'Schéma réel')); ui.wait('TABLE extra'); ui.send('j'); ui.wait('TABLE products'); ui.wait('stock_id → stocks.id')
            ui.send('d'); ui.wait(label('HTTP response', 'Réponse HTTP'))

        if json.loads((local_pack / 'manifest.json').read_text())['schema_version'] >= 3:
            config_before = (project / 'miblo.yaml').read_bytes()
            ui.send('g'); ui.wait(label('Create a route — linked model', 'Créer une route — modèle associé'))
            ui.send('\x1b'); ui.wait_absent(label('Create a route — linked model', 'Créer une route — modèle associé'))
            assert (project / 'miblo.yaml').read_bytes() == config_before
            ui.send('g'); ui.wait(label('Create a route — linked model', 'Créer une route — modèle associé'))
            ui.send('\r'); ui.wait(label('Route action for this model', 'Action de la route pour ce modèle'))
            ui.send('\x1b[B' * 5 + '\r'); ui.wait(label('New route method', 'Méthode de la nouvelle route'))
            ui.send('\r'); ui.wait(label('Route path', 'Chemin de la route'))
            ui.send('\x15/products/active\r'); ui.wait(label('Route / function name', 'Nom de la route / fonction'))
            ui.send('\x15active_products\r'); ui.wait(label('Create this route?', 'Créer cette route ?')); ui.wait(label('501 not implemented', '501 non implémenté'))
            ui.send('\r'); ui.wait(label('Route creation : exit status: 0', 'Création de route : exit status: 0')); ui.wait(label('No action running', 'Aucune action en cours'))
            ui.send('e'); ui.wait(label('status', 'État'))
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline and 'custom_routes/active_products/service.rs' not in marker.read_text(): ui.pump()
            assert marker.read_text().endswith('/custom_routes/active_products/service.rs')

        model_routes = 'model-routes' in json.loads((local_pack / 'manifest.json').read_text()).get('features', [])
        if model_routes:
            ui.send('g'); ui.wait(label('Create a route — linked model', 'Créer une route — modèle associé'))
            ui.send('\r'); ui.wait(label('Route action for this model', 'Action de la route pour ce modèle'))
            ui.send('\x1b[B' * 2 + '\r'); ui.wait(label('Route path', 'Chemin de la route'))
            ui.send('\x15/products/quick\r'); ui.wait(label('Route / function name', 'Nom de la route / fonction'))
            ui.send('\x15quick_create\r'); ui.wait(label('Create this route?', 'Créer cette route ?')); ui.wait(label('model SQL', 'SQL du modèle'))
            # A modified local checkout must keep the form and offer recovery.
            asset = local_pack / 'route/handlers.rs.hbs'
            original_asset = asset.read_bytes()
            asset.write_bytes(original_asset + b'\n')
            ui.send('\r'); ui.wait(label('Template update required', 'Template à mettre à jour'))
            assert not (project / 'src/resources/custom_routes/quick_create').exists()
            asset.write_bytes(original_asset)
            ui.send('\x1b[B\r'); ui.wait(label('Create this route?', 'Créer cette route ?')); ui.wait('/products/quick')
            ui.send('\r'); ui.wait(label('model CRUD and SQL connected', 'CRUD et SQL du modèle reliés')); ui.wait(label('No action running', 'Aucune action en cours'))
            alias = project / 'src/resources/custom_routes/quick_create'
            assert 'product::handlers::create' in (alias / 'handlers.rs').read_text()
            assert not (alias / 'service.rs').exists()

        ui.send('r'); ui.wait(label('API running', 'API en cours') if args.api_target_dir else label('API not responding', 'API ne répond pas'), timeout=120)
        if args.api_target_dir:
            with urllib.request.urlopen(f'http://127.0.0.1:{api_port}/products/{old_id}') as response:
                row = json.load(response)
            assert row['title'] == 'before-edit' and row['alias'] is None and row['stock_id'] is None
            if model_routes:
                with sqlite3.connect(project / 'miblo.sqlite') as connection:
                    count_before = connection.execute('SELECT COUNT(*) FROM products').fetchone()[0]
                ui.send('x'); ui.wait(label('Prepare request', 'Préparer la requête')); ui.wait('/products/quick')
                ui.send('\r'); ui.wait('HTTP 201')
                with sqlite3.connect(project / 'miblo.sqlite') as connection:
                    assert connection.execute('SELECT COUNT(*) FROM products').fetchone()[0] == count_before + 1
                ui.send(':routes\r'); ui.send('k')
            if (project / 'src/resources/custom_routes').exists():
                ui.send('x'); ui.wait(label('HTTP path', 'Chemin HTTP')); ui.wait('/products/active')
                ui.send('\r'); ui.wait('HTTP 501'); ui.wait(label('action to implement', 'action à implémenter')); ui.wait('service.rs')
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
