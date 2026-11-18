#!/usr/bin/env python3
"""Check English defaults, live switching, persistence and failed preference writes.

Uses an isolated user config directory and a PTY. Requires the same pyte setup as
smoke_tui.py; no API, database server or network is needed.
"""
import argparse
import json
import os
from pathlib import Path
import tempfile
from tui_terminal import Terminal

parser = argparse.ArgumentParser()
parser.add_argument('--miblo', type=Path, required=True)
parser.add_argument('--capture-json', type=Path)
args = parser.parse_args()

with tempfile.TemporaryDirectory(prefix='miblo-language-') as temporary:
    root = Path(temporary)
    config = root / 'preferences'
    settings = config / 'settings.json'
    env = dict(os.environ, TERM='xterm-256color', LANG='fr_FR.UTF-8',
               MIBLO_CONFIG_DIR=str(config), MIBLO_NO_ANIMATION='1')
    env.pop('MIBLO_LANGUAGE', None)
    binary = str(args.miblo.resolve())

    ui = Terminal(binary, root, env)
    try:
        ui.wait('Welcome')
        ui.wait('API workspace')
        ui.wait('EN · L Language/Langue')
        assert not settings.exists(), 'Starting Miblo should not create preferences'
        ui.send('L')
        ui.wait('Applies immediately')
        ui.send('\x1b')
        ui.wait_absent('Applies immediately')
        ui.wait('API workspace')
        assert not settings.exists(), 'Cancelling must not save a preference'
        ui.send('L'); ui.wait('Applies immediately')
        ui.send('\x1b[B\r'); ui.wait('Langue enregistrée : français')
        ui.wait('Atelier API'); ui.wait('FR · L Language/Langue')
        assert json.loads(settings.read_text())['language'] == 'fr'
        ui.send('n'); ui.wait('Nom de la nouvelle API'); ui.send('\x1b')
        ui.wait_absent('Nom de la nouvelle API')
        ui.send('?'); ui.wait('Aide Miblo'); ui.send('\x1b')
        ui.wait_absent('Aide Miblo')
    finally:
        ui.close()

    # The saved choice is read before the next workspace is rendered.
    ui = Terminal(binary, root, env)
    try:
        ui.wait('Bienvenue'); ui.wait('Atelier API')
        ui.send(' '); ui.wait('Actions')
        ui.send('\x1b[A\r'); ui.wait('Appliqué immédiatement')
        ui.send('\x1b[A\r'); ui.wait('Language saved: English')
        assert json.loads(settings.read_text())['language'] == 'en'
        ui.send(':language\r'); ui.wait('Applies immediately'); ui.send('\x1b')
        ui.wait_absent('Applies immediately')
        ui.send(':langue\r'); ui.wait('Applies immediately'); ui.send('\x1b')
        ui.wait_absent('Applies immediately')
        ui.resize(70, 20)
        for _ in range(4): ui.pump(.1)
        ui.wait('EN · L Language/Langue')
        ui.send('L'); ui.wait('English'); ui.wait('Français'); ui.send('\x1b')
        ui.wait_absent('Français')
        ui.resize(130, 40)
        for _ in range(4): ui.pump(.1)
        ui.wait('API workspace')
        if args.capture_json:
            ui.send('L'); ui.wait('Applies immediately')
            ui.capture(args.capture_json)
    finally:
        ui.close()

    # Environment overrides are explicit and don't rewrite the saved choice.
    ui = Terminal(binary, root, dict(env, MIBLO_LANGUAGE='fr'))
    try:
        ui.wait('Bienvenue'); ui.send('L'); ui.wait('MIBLO_LANGUAGE prime')
        assert json.loads(settings.read_text())['language'] == 'en'
    finally:
        ui.close()

    # Invalid settings must not prevent startup or be silently replaced.
    settings.write_text('{invalid-json')
    ui = Terminal(binary, root, env)
    try:
        ui.wait('Welcome'); ui.send('L'); ui.wait('Applies immediately')
        ui.send('\x1b[B\r'); ui.wait('Langue changée pour cette session')
        ui.wait('Atelier API')
        assert settings.read_text() == '{invalid-json'
    finally:
        ui.close()

    # A configuration path that is a file reproduces an unwritable destination.
    blocked = root / 'not-a-directory'
    blocked.write_text('keep this file')
    ui = Terminal(binary, root, dict(env, MIBLO_CONFIG_DIR=str(blocked)))
    try:
        ui.wait('Welcome'); ui.send('L'); ui.wait('Applies immediately')
        ui.send('\x1b[B\r'); ui.wait('Langue changée pour cette session')
        ui.wait('Atelier API')
        assert blocked.read_text() == 'keep this file'
    finally:
        ui.close()

print('PASS: English default, EN/FR menu, instant switch, persistence, cancellation, compact terminal and preference errors')
