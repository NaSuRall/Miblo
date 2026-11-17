"""Shared real-database relation checks for SQLite and MySQL smoke tests."""
import json
import subprocess


def add_models(cli):
    for name, fields in [
        ('Group', ['name:string']),
        ('Member', ['name:string', 'group_id:int:ref=Group']),
        ('GroupNote', ['body:text', 'group_id:int:ref=Group:on_delete=cascade']),
        ('GroupBadge', ['label:string', 'group_id:int?:unique:ref=Group:on_delete=set_null']),
        ('Folder', ['name:string', 'parent_id:int?:ref=Folder:on_delete=set_null']),
    ]:
        cli('add', name, *fields, '--yes', '--offline')


def inspect_schema(miblo, project, env):
    result = subprocess.run([miblo, 'db', 'schema'], cwd=project, env=env, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    schema = json.loads(result.stdout)
    tables = {table['name']: table for table in schema['tables']}
    assert '_sqlx_migrations' not in tables
    for table, action in [('members', 'RESTRICT'), ('group_notes', 'CASCADE'), ('group_badges', 'SET NULL')]:
        data = tables[table]
        assert data['columns'][0]['primary_key'] == 1
        key = data['foreign_keys'][0]
        assert key['target'] == 'groups' and key['target_columns'] == ['id']
        assert key['columns'] == ['group_id'] and key['on_delete'] == action, key
    assert tables['folders']['foreign_keys'][0]['target'] == 'folders'
    return schema


def requests(request):
    group = request('POST', '/groups', {'name': 'team'}, 201)
    gid = group['id']
    request('POST', '/members', {'name': 'orphan', 'group_id': 999999}, 400)
    member = request('POST', '/members', {'name': 'valid', 'group_id': gid}, 201)
    mid = member['id']
    request('PATCH', f'/members/{mid}', {'group_id': 999999}, 400)
    assert request('GET', f'/members/{mid}')['group_id'] == gid
    request('DELETE', f'/groups/{gid}', expected=400)
    assert request('GET', f'/groups/{gid}')['id'] == gid
    note = request('POST', '/group_notes', {'body': 'cascade', 'group_id': gid}, 201)
    badge = request('POST', '/group_badges', {'label': 'nullable', 'group_id': gid}, 201)
    request('POST', '/group_badges', {'label': 'duplicate', 'group_id': gid}, 409)
    request('DELETE', f'/members/{mid}', expected=204)
    request('DELETE', f'/groups/{gid}', expected=204)
    request('GET', f'/group_notes/{note["id"]}', expected=404)
    assert request('GET', f'/group_badges/{badge["id"]}')['group_id'] is None
    parent = request('POST', '/folders', {'name': 'root', 'parent_id': None}, 201)
    child = request('POST', '/folders', {'name': 'child', 'parent_id': parent['id']}, 201)
    request('DELETE', f'/folders/{parent["id"]}', expected=204)
    assert request('GET', f'/folders/{child["id"]}')['parent_id'] is None
    print('PASS: actual PK/FK schema, orphan rejection, protected delete, cascade, SET NULL, unique reference and self relation', flush=True)
