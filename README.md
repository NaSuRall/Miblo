# Miblo CLI

Create and extend a Rust REST API using Axum, SQLx and SQLite or MySQL. The CLI asks for
project settings and resources, downloads a versioned Handlebars template pack,
and writes a normal Rust project that runs independently of Miblo.

This development version is **0.3.0**. It uses template format **1** with
`rust-axum-mysql` **0.1.0** or `rust-axum-sqlite` **0.2.0**. Neither publishing a crate nor publishing a GitHub
Release is performed by the build or packaging scripts.

## Install from this checkout

```sh
cargo install --path . --locked
miblo --help
```

The executable is `miblo`; the Cargo package remains `miblo_cli_v2`.
A recent stable Rust toolchain is required. SQLite needs no external database
server. MySQL projects need a reachable MySQL server to run their APIs.
There is no dependency on an installed `sqlx-cli`.

Once this version is published, installation will be:

```sh
cargo install miblo_cli_v2 --version 0.3.0 --locked
```

## Full-screen API workspace

Run `miblo` in a terminal to open the workspace. It recognizes a Miblo project
in the current directory. You can also open a specific project directly:

```sh
miblo
miblo ui --project path/to/my-api
```

The workspace includes a resource list, file tree, read-only code preview,
project/API status and bounded process logs. It supports three themes for the
current session and adapts its panels to the terminal size (minimum 70 × 20).

| Key | Action |
| --- | --- |
| Arrows or `j` / `k` | Navigate or scroll |
| `Tab` / `Shift+Tab` | Switch panels |
| `Enter` | Open a file, folder or resource |
| `Space` | Open the action palette |
| `:` | Enter a command |
| `n` / `o` / `a` | New API / open project / add model + SQL table + CRUD |
| `b` | Database connection, initialization and migrations |
| `r` / `s` | Start / stop the API |
| `c` | Run `cargo check` |
| `e` | Open the selected file in an external editor |
| `t` / `F5` | Change theme / refresh project |
| `?` / `q` | Help / quit |

Commands include `:new`, `:open path/to/project`, `:add`, `:run`, `:stop`,
`:db`, `:migrate`, `:check`, `:test`, `:doctor`, `:edit`, `:refresh`, `:theme`, `:help`, `:quit`.
Creation and resource addition use popup forms with arrow-key selections and
Space to toggle field options. Escape cancels a form before generation.
Project creation asks for the database (SQLite by default), then the template source: GitHub release, local pack or
custom archive URL. The UI suggests a valid pack found at
`miblo_templates/rust_template_v1` for MySQL or
`miblo_templates/rust_sqlite_template_v1` for SQLite, relative to the current
directory or its parents (the folder directly inside a template checkout also
works). A matching `MIBLO_TEMPLATE_DIR` takes priority when set.
The suggested local path is preselected and remains reviewable before generation;
no template content or development-machine path is embedded in the binary.
Choose **GitHub Releases** to download the published pack without a local checkout,
or **Dossier local** when developing templates.
If creation fails, the UI offers a source change or retry while preserving the
project name, destination and port. A missing release (HTTP 404) writes no project.
Creation starts an empty API;
add its resources from the same workspace afterwards.

The code view is intentionally read-only. `e` uses `VISUAL`, then `EDITOR`,
then `vi` on Unix / `notepad` on Windows. Editor arguments are supported without
invoking a shell (for example `EDITOR="code --wait"`). Miblo restores the normal
terminal while the editor is open and refreshes the code when it returns.
Private `.env` files are omitted from the explorer; `.env.example` is visible.

The API and Cargo checks run in background processes, with output in the Logs
panel. Miblo distinguishes startup from the generated server's ready message.
The database panel reports connection checks and migration results separately. Stop and normal quit
terminate processes started by this workspace, including Cargo's API child.
Finish generation/check jobs before quitting; the UI blocks quit while those
jobs are active to avoid interrupting file writes. SQLite setup is automatic;
configure MySQL from the `b` menu before starting its API.

Without an interactive terminal, bare `miblo` displays help instead of waiting
for input. Explicit commands remain available for scripts and automation.

## Models and database setup from the interface

Press `a` to add a model such as `User`. Select its fields and options (nullable,
unique, private), then confirm: Miblo generates the Rust model, SQL migration and
CRUD routes. Existing application code is preserved. Changes to an existing
model/schema still need handwritten migrations; the interface adds new models.

**SQLite:** Miblo creates `.env`, initializes `miblo.sqlite`, and applies migrations
after creating a project and after adding models while the API is stopped.
Standalone `cargo run` also creates the file if missing and runs pending migrations.
The database is persistent, with no server/container/SQLx CLI to install.

**MySQL:** after project creation, the UI asks for host, port, database, username
and a masked password. It saves the connection in the project's private `.env`,
preserves other settings, and tests the connection. An existing MySQL server must
be reachable; Miblo does not install the server. From `b`, select **Initialiser la
base et ses tables** to create the database if needed (requires appropriate user
permissions), or **Appliquer les migrations** for new tables. Both actions ask
for confirmation and require the API started by Miblo to be stopped.

The same actions are scriptable without putting passwords in command arguments:

```sh
miblo db check --project my-api
miblo db init --project my-api
miblo db migrate --project my-api
```

The `b` menu can update MySQL settings later. Miblo uses the project's `.env`
connection, then `DATABASE_URL` from its environment if the file does not define
it. The database engine is fixed at project creation; changing an existing
project from MySQL to SQLite requires migrating the code and data separately.

## Guided creation

```sh
miblo new shop-api
```

The direct-command wizard also uses arrow-key menus and Space to select field
options. It asks for the HTTP port, resource names, fields and collection URLs.
It also asks for SQLite (default) or MySQL. `--database sqlite|mysql` sets the
choice explicitly; scripts using `--yes` without this flag keep the previous
MySQL default for compatibility.
Authentication is deliberately deferred.

The default template URLs depend on the chosen database:

```text
https://github.com/NaSuRall/miblo_templates/releases/download/v0.1.0/rust-axum-mysql.tar.gz
https://github.com/NaSuRall/miblo_templates/releases/download/v0.2.0/rust-axum-sqlite.tar.gz
```

Each archive has a `.sha256` sidecar in its versioned GitHub release.
No local template folder is needed for the default remote command.
For template development, with both repositories checked out next to one another:

```sh
miblo new shop-api --database mysql --template-dir ../miblo_templates/rust_template_v1
```

Do not point this option at the legacy `rust_template/config.yaml`: the new
pack has a `manifest.json` and separate `project/` and `resource/` templates.
`MIBLO_TEMPLATE_DIR` can set a local development source once per shell session.

## Noninteractive creation

```sh
miblo new shop-api \
  --template-dir ../miblo_templates/rust_template_v1 \
  --resource Product \
  --field title:string:unique \
  --field 'description:text?' \
  --field price:float \
  --field active:bool \
  --yes
```

`--yes` accepts supplied options and defaults. Without `--resource`, it creates
an empty API with `/health`. `--output` selects a destination directory;
existing destinations are always refused. `--dry-run` shows the planned files
without creating the project (it may download/cache the selected template).

Field syntax is `name:type`, with `?` for nullable and `:unique` or `:private`
flags. Field names use snake_case. Resource names use PascalCase. `id` is an
automatically generated signed 64-bit integer key and cannot be supplied as a field.

| Type | Rust | MySQL | SQLite |
| --- | --- | --- | --- |
| `string` | `String` | `VARCHAR(255)` | `TEXT` |
| `text` | `String` | `TEXT` | `TEXT` |
| `int` | `i64` | `BIGINT` | `INTEGER` |
| `bool` | `bool` | `BOOLEAN` | `INTEGER` |
| `float` | `f64` | `DOUBLE` | `REAL` |

Nullable fields use `Option<T>`. `text:unique` is rejected; use `string:unique`.
Private fields are accepted on writes and omitted from API responses. Common
sensitive names (`password`, `password_hash`, `secret`, `token`, `api_key`) are
automatically private when supplied through the CLI. This is output filtering,
not encryption, password hashing or authentication.

## Run the API

```sh
cd shop-api
cp .env.example .env
docker compose up -d db
# Wait for MySQL to be healthy.
cargo run
```

An existing MySQL server also works: configure `DATABASE_URL` in `.env` or the
environment. The Compose file and its example credentials are for local use.
Migrations are applied by the API at startup. `miblo run` runs `cargo run` and
preserves its stdout, stderr and failure status. `miblo run --project PATH`
selects a project from another directory.

Generated projects include a lockfile for dependency reproducibility, a
liveness test and a PATCH deserialization test. SQL uses bound parameters and
runtime SQLx queries, so compilation does not need a live database or a `.sqlx`
metadata directory. SQL behavior is covered separately by the MySQL smoke test.

## Add a resource without losing custom code

```sh
miblo add Category label:string --yes
# Or use the guided prompt:
miblo add Product
```

The original `miblo add resource Category --field label:string --yes` syntax
remains supported. Do not mix positional fields with `--field`.

Use `--path /categories` to override the default collection path. The default
uses the snake_case name with simple pluralization; it is not a full English
inflection engine. Use `--project PATH` outside the generated project.

Miblo creates only the new resource's module, model, handlers and migration.
Existing resource files and migrations are not regenerated. It updates:

- `src/resources/mod.rs`: generated route registration;
- `miblo.yaml`: project description;
- `.miblo/state.json`: pinned template and consistency metadata.

Customize resource files freely and register handwritten routes in `src/lib.rs`.
Whitespace-only registry formatting is accepted. If registry content changes,
Miblo stops instead of overwriting it. Restore that generated registry after
moving custom registration to `src/lib.rs` to resume additions.

An existing output file, modified project definition, changed local template,
symlinked output or concurrent operation causes an error. Additions are prepared
and conflicts checked before writing. Normal I/O failures trigger rollback.
A process interruption can leave a partial operation and `.miblo/operation.lock`;
inspect the working tree and restore it from Git before removing that lock.
The multi-file operation is not an atomic database transaction.

`--dry-run` previews additions. `--offline` reuses a cached remote pack.
Commit `miblo.yaml` and `.miblo/state.json` along with your code. A local template
path is a development option and must remain available and unchanged. A remote
pack is pinned by URL, version, SHA-256 and content fingerprint for later additions.

## API behavior

Every resource includes list, get-one, create, update and delete endpoints.
The list supports `?limit=20&offset=0`, with a maximum limit of 100.
Creation returns 201; uniqueness conflicts 409; missing rows 404; deletion 204.
Unknown JSON fields and incorrect types are rejected. PATCH distinguishes an
omitted field from explicit null, accepts null only for nullable fields, and
rejects attempts to change `id`. Private fields never appear in normal resource
JSON responses. The scaffold has no authentication or authorization yet.

## Templates on your own server

```sh
miblo new shop-api --template-base-url https://templates.example.com --yes
# Fetches /v0.1.0/rust-axum-mysql.tar.gz and its .sha256 sidecar.

miblo new shop-api \
  --template-url https://templates.example.com/custom/api.tar.gz \
  --template-version 0.1.0 --yes
```

`MIBLO_TEMPLATE_BASE_URL` can set the default base URL. `--sha256 HEX` provides a
trusted digest directly instead of fetching the sidecar. Templates must be served
over HTTPS; HTTP on loopback is supported for local tests. Archives are bounded
in size and cannot contain links, unexpected files or escaping paths. Downloaded
code still comes from the template publisher you choose to trust; a checksum
sidecar on the same server is not an independent publisher signature.

`--cache-dir PATH` / `MIBLO_CACHE_DIR` changes the cache location. Otherwise it
uses XDG_CACHE_HOME, LOCALAPPDATA or `~/.cache/miblo`. URL/version pairs are pinned
after the first successful download; release a new template version to change a
pack. A corrupt cached archive is refused; delete that archive and download again.

## Configuration and compatibility

`miblo.yaml` stores schema version 1, project name, stack, port and resources.
To create another project from an edited copy of this definition:

```sh
miblo new --config path/to/miblo.yaml --template-dir PATH --yes
```

Editing an existing resource definition in place does not update its SQL table.
For this release, make schema changes through handwritten migrations and code.
The `add resource` command checks that existing definitions still match its state.
Set runtime port and database options through the generated `.env`.

The old `init --name ... --template-dir config.yaml` command and its YAML schema
are not supported in 0.3. Projects made with 0.2 remain supported. Existing generated applications remain ordinary Rust
projects and continue to run on their own. Use a separate destination to try a new database engine;
there is no automatic upgrade of old projects or templates.

## Development and checks

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The CLI tests include an isolated HTTP fixture for download/cache behavior.
To test the actual template output with MySQL installed (`mysql` and `mysqld`):

```sh
cargo build --locked
python3 scripts/smoke_mysql.py \
  --miblo target/debug/miblo \
  --templates ../miblo_templates/rust_template_v1
```

This script creates its own temporary MySQL data directory and local ports,
compiles a generated API, exercises CRUD and cleans up its processes and data.
It never uses an existing database. Docker is not required for this test.

Use `--remote` instead of `--templates ...` to verify the default published
release with an empty template cache. That mode clears template source overrides,
downloads and pins the public archive, adds another resource offline from the
cache, then runs the same generated-API tests against isolated MySQL.

SQLite has a separate end-to-end test, with no database server prerequisite:

```sh
python3 scripts/smoke_sqlite.py --miblo target/debug/miblo \
  --templates ../miblo_templates/rust_sqlite_template_v1 \
  --target-dir target/sqlite-smoke
```

It checks empty-project startup, automatic file creation, model additions,
migrations, CRUD, concurrent PATCH and persistence across restarts. `--remote`
replaces the local template option to verify the published SQLite pack.

A PTY smoke test exercises the real full-screen UI on Unix/macOS. It requires
`pyte` in a separate Python environment and uses fake Cargo/editor processes
instead of a database:

```sh
python3 scripts/smoke_tui.py \
  --miblo target/debug/miblo \
  --database sqlite --templates ../miblo_templates/rust_sqlite_template_v1
```

The CLI is organized around `config`, `templates`, `generation`, `wizard`,
`cli`, `database`, `runtime` and `tui`. The production template pack stays in the separate
`miblo_templates` repository; it is not embedded in the CLI binary.

## Next steps

Authentication with protected routes and proper password handling, richer field
types/relations, OpenAPI, existing-schema evolution, then one Node/TypeScript
backend. These are not implemented by this release.

License: MIT.
