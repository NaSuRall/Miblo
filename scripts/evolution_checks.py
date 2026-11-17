"""Checks shared by the real SQLite and MySQL model-evolution smoke tests."""
import json
import re
import subprocess


def generate(miblo, project, env, model):
    migrations = {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')}
    folder = project / 'src/resources' / model.lower()
    layered = (folder / 'requests.rs').exists()
    source = folder / ('requests.rs' if layered else 'handlers.rs')
    if layered:
        service = folder / 'service.rs'
        # An observable business rule must survive model evolution byte-for-byte.
        service.write_text(service.read_text().replace('request.validate()?;', 'request.validate()?;\n    if request.' + ('name' if model == 'User' else 'title') + ' == "blocked-by-service" { return Err(ApiError::bad_request("Business rule")); }', 1))
    # A real business-logic edit, not only a comment at the end of the file.
    content = source.read_text()
    default_limit = r'(fn default_limit\(\) -> u32\s*\{\s*)20'
    assert re.search(default_limit, content)
    source.write_text(re.sub(default_limit, r'\g<1>17', content))
    subprocess.run(['cargo', 'fmt'], cwd=project, env=env, check=True)
    service_before = (folder / 'service.rs').read_bytes() if layered else None
    handler_before = (folder / 'handlers.rs').read_bytes() if layered else None
    before = source.read_bytes()
    def cli(*arguments):
        result = subprocess.run([miblo, *arguments], cwd=project, env=env, capture_output=True, text=True)
        assert result.returncode == 0, (arguments, result.stdout, result.stderr)
        return result
    fields = ['alias:string?:unique', 'category_id:int?:ref=Category:on_delete=set_null']
    plan = cli('edit', model, *fields, '--dry-run', '--offline').stdout
    assert 'ALTER TABLE' in plan and 'CREATE TABLE' not in plan and 'DROP TABLE' not in plan
    assert source.read_bytes() == before
    assert {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')} == migrations
    cli('edit', model, *fields, '--yes', '--offline')
    cli('edit', model, 'ranking:float?', 'verified:bool?', 'internal_note:text?:private', '--yes', '--offline')
    current = source.read_text()
    assert re.search(default_limit.replace('20', '17'), current) and 'alias' in current and 'category_id' in current
    assert 'ranking' in current and 'verified' in current and 'internal_note' in current
    for path, content in migrations.items():
        assert path.read_bytes() == content, 'An existing migration was rewritten'
    backups = list((project / '.miblo/backups').glob('*/src/resources/' + model.lower() + '/' + source.name))
    assert len(backups) == 2 and any(p.read_bytes() == before for p in backups)
    cli('doctor')
    # A new model after editing must preserve the baseline of the edited model.
    cli('add', 'AfterEdit', 'label:string', '--yes', '--offline')
    state = json.loads((project / '.miblo/state.json').read_text())
    key = 'src/resources/' + model.lower() + '/' + source.name
    assert re.search(default_limit, state['generated'][key])
    if layered:
        assert (folder / 'service.rs').read_bytes() == service_before
        assert (folder / 'handlers.rs').read_bytes() == handler_before
        assert 'INSERT INTO' not in (folder / 'handlers.rs').read_text()
        subprocess.run(['cargo', 'fmt', '--check'], cwd=project, env=env, check=True)
    print('PASS: additive edit plans, repeated custom-code merges, backups and unchanged migration history', flush=True)


def requests(request, collection, existing_path, value, layered=False):
    if layered:
        blocked = dict(value)
        blocked['name' if 'name' in value else 'title'] = 'blocked-by-service'
        request('POST', collection, blocked, 400)
    old = request('GET', existing_path)
    for name in ['alias', 'category_id', 'ranking', 'verified']:
        assert old[name] is None, (name, old)
    assert 'internal_note' not in old
    for key, expected in value.items():
        if key != 'token':
            assert old[key] == expected, (key, old[key], expected)
    parent = request('POST', '/categories', {'label': 'linked after migration'}, 201)
    patch = dict(alias='unique-after-edit', category_id=parent['id'], ranking=2.5, verified=True, internal_note='private')
    result = request('PATCH', existing_path, patch)
    assert result['alias'] == patch['alias'] and result['category_id'] == parent['id']
    assert result['ranking'] == 2.5 and result['verified'] is True and 'internal_note' not in result
    request('PATCH', existing_path, {'category_id': 9999999}, 400)
    # New optional fields may still be omitted by old clients.
    another = dict(value)
    key = 'email' if 'email' in value else 'title'
    another[key] = 'another@example.test' if key == 'email' else 'another-after-edit'
    created = request('POST', collection, another, 201)
    request('PATCH', f'{collection}/{created["id"]}', {'alias': patch['alias']}, 409)
    request('DELETE', f'/categories/{parent["id"]}', expected=204)
    assert request('GET', existing_path)['category_id'] is None
    request('PATCH', existing_path, {'alias': None, 'ranking': None, 'verified': None})
    print('PASS: existing values/ids preserved, NULL backfill, evolved CRUD, uniqueness, privacy and foreign keys', flush=True)
