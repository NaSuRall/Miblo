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
    assert service.read_text() == code
    assert json.loads((project / '.miblo/state.json').read_text())['next_migration'] == state['next_migration']
    assert {p: p.read_bytes() for p in (project / 'migrations').glob('*.sql')} == migrations
    return True


def requests(request, collection):
    result = request('POST', '/custom/a-slug/echo?filter=active', {'example': 42})
    assert result == {'slug': 'a-slug', 'query': {'filter': 'active'}, 'body': {'example': 42}, 'database_value': 1}, result
    request('POST', '/custom/a-slug/echo', expected=400)
    for method in ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']:
        request(method, '/custom/' + method.lower(), expected=501)
    request('HEAD', '/custom/get', expected=501)
    request('GET', collection + '/active', expected=501)
    print('PASS: custom routes, all HTTP methods, path/query/JSON extraction, SQL pool, 501 stubs, code and migration preservation', flush=True)
