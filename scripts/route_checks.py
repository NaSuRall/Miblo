"""Custom route checks shared by the SQLite/MySQL end-to-end smoke tests."""
import json


def generate(cli, project, resource, collection):
    state = json.loads((project / '.miblo/state.json').read_text())
    if tuple(map(int, state['template']['version'].split('.'))) < (0, 5, 0):
        return False
    migrations = {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')}
    cli('route', 'add', 'echo', '--method', 'POST', '--path', '/custom/{slug}/echo', '--json-body', '--yes', '--offline')
    service = project / 'src/resources/custom_routes/echo/service.rs'
    code = service.read_text()
    for name in ['pool', 'params', 'query', 'body']:
        code = code.replace('_' + name + ':', name + ':')
    code = code.replace('Err(RouteError::NotImplemented)', 'let database_value: (i64,) = sqlx::query_as("SELECT 1").fetch_one(pool).await?;\n    Ok(serde_json::json!({"slug": params.slug, "query": query, "body": body, "database_value": database_value.0}))')
    service.write_text(code)
    for method in ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']:
        cli('route', 'add', 'custom_' + method.lower(), '--method', method, '--path', '/custom/' + method.lower(), '--yes', '--offline')
    cli('route', 'add', 'active_items', '--path', collection + '/active', '--resource', resource, '--yes', '--offline')
    automatic = tuple(map(int, state['template']['version'].split('.'))) >= (0, 6, 0)
    if automatic:
        for action in ['list', 'get', 'create', 'update', 'delete']:
            path = '/automatic' + collection + ('/{id}' if action in ['get', 'update', 'delete'] else '')
            cli('route', 'add', 'model_' + action, '--resource', resource, '--action', action, '--path', path, '--yes', '--offline')
            alias = project / ('src/resources/custom_routes/model_' + action)
            assert (alias / 'handlers.rs').exists() and not (alias / 'service.rs').exists()
    assert service.read_text() == code
    assert json.loads((project / '.miblo/state.json').read_text())['next_migration'] == state['next_migration']
    assert {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')} == migrations
    return 'model-routes' if automatic else 'custom-routes'


def requests(request, collection):
    result = request('POST', '/custom/a-slug/echo?filter=active', {'example': 42})
    assert result == {'slug': 'a-slug', 'query': {'filter': 'active'}, 'body': {'example': 42}, 'database_value': 1}, result
    request('POST', '/custom/a-slug/echo', expected=400)
    for method in ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']:
        request(method, '/custom/' + method.lower(), expected=501)
    request('HEAD', '/custom/get', expected=501)
    request('GET', collection + '/active', expected=501)
    print('PASS: custom routes, all HTTP methods, path/query/JSON extraction, SQL pool, 501 stubs, code and migration preservation', flush=True)


def model_requests(request, collection, value):
    """Exercise actual SQL through all five aliases, with no handwritten route code."""
    alias = '/automatic' + collection
    created = request('POST', alias, value, 201)
    assert 'token' not in created
    path = alias + '/' + str(created['id'])
    canonical = collection + '/' + str(created['id'])
    assert request('GET', path) == request('GET', canonical)
    assert any(row['id'] == created['id'] for row in request('GET', alias + '?limit=100'))
    request('GET', alias + '?limit=101', expected=400)
    request('GET', alias + '/invalid-id', expected=400)
    request('POST', alias, value, 409)
    request('POST', alias, dict(value, id=77), 422)
    request('PATCH', path, {}, 400)
    changed = request('PATCH', path, {'active': False})
    assert changed['active'] is False and request('GET', canonical)['active'] is False
    request('DELETE', path, expected=204)
    request('GET', canonical, expected=404)
    request('GET', path, expected=404)
    request('DELETE', path, expected=404)
    print('PASS: model routes execute real SQL CRUD, validation, pagination, private fields and shared data', flush=True)
