# Miblo CLI

Create and extend a Rust REST API using Axum, SQLx and SQLite or MySQL. The CLI asks for
project settings and resources, downloads a versioned Handlebars template pack,
and writes a normal Rust project that runs independently of Miblo.

This development version is **0.9.0**. It supports template formats **1, 2 and 3**, with new projects using format **3** and
`rust-axum-mysql` or `rust-axum-sqlite`, both version **0.6.0**. Neither publishing a crate nor publishing a GitHub
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
cargo install miblo_cli_v2 --version 0.9.0 --locked
```

## Full-screen API workspace

Run `miblo` in a terminal to open the workspace. It recognizes a Miblo project
in the current directory. You can also open a specific project directly:

```sh
miblo
miblo ui --project path/to/my-api
```

A blue MIBLO wordmark is written on screen at startup (under one second). Any key skips it; Ctrl+C quits. Set `MIBLO_NO_ANIMATION=1` to disable it. Scriptable commands never show the animation.

The workspace includes a resource list, file tree, route tester, API/database
status cards, a small read-only code preview and bounded process logs. It supports three themes for the
current session and adapts its panels to the terminal size (minimum 70 × 20).

| Key | Action |
| --- | --- |
| Arrows or `j` / `k` | Navigate or scroll |
| `Tab` / `Shift+Tab` | Switch panels |
| `Enter` | Open a file, folder or resource; test the selected route |
| `Space` | Open the action palette |
| `:` | Enter a command |
| `n` / `o` / `a` | New API / open project / add model + SQL table + CRUD |
| `b` | Database connection, initialization and migrations |
| `d` | View database schema / return to routes |
| `p` (schema view) | Switch between actual database and declared models |
| `u` | Update templates used for future models and routes |
| `r` / `s` | Start / stop the API |
| `c` | Run `cargo check` |
| `g` | Create a model CRUD route or custom business function |
| `x` | Test the selected route with a prefilled field form |
| `f` | Custom HTTP request: GET, POST, PATCH, PUT, DELETE, HEAD or OPTIONS |
| `e` | Open the selected file or route code in an external editor |
| `m` | Add fields / relations to an existing model |
| `v` | Show / hide the six-line code preview |
| `t` / `F5` | Change theme / refresh project |
| `?` / `q` | Help / quit |

Commands include `:new`, `:open path/to/project`, `:add`, `:model`, `:run`, `:stop`,
`:db`, `:migrate`, `:schema`, `:templates`, `:routes`, `:route-new`, `:route`, `:request`, `:code`, `:check`, `:test`, `:doctor`, `:edit`, `:refresh`, `:theme`, `:help`, `:quit`.
Creation and resource addition use popup forms with arrow-key selections and
Space to toggle field options. Escape cancels a form before generation.
Project creation asks for the database (SQLite by default), then the template source: GitHub release, local pack or
custom archive URL. The UI suggests a valid pack found at
`miblo_templates/rust_template_v4` for MySQL or
`miblo_templates/rust_sqlite_template_v4` for SQLite, relative to the current
directory or its parents (the folder directly inside a template checkout also
works). A matching `MIBLO_TEMPLATE_DIR` takes priority when set.
GitHub Releases is selected by default; the local path is available when explicitly chosen;
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
panel. Status cards stay visible even in compact terminals:

- **Green:** the API answers `GET /health` successfully; the database is reachable
  and its migration history matches the project's SQL files.
- **Orange:** startup, a required API restart, database initialization, or pending
  SQL migrations. Press `b` to review and apply migrations.
- **Red:** the API process exited unexpectedly, health responds with an HTTP error,
  a started API no longer responds, the database is unreachable, or an applied
  migration has changed or disappeared.
- **Gray:** stopped API, unopened project or a check still pending.

Checks run in the background approximately every three seconds, with bounded
HTTP/database timeouts. Database status reads SQLx history without creating a
missing database or applying migrations. Reopening a project checks its real
state again. An API started elsewhere is marked as external; Miblo does not stop
that process. The API address follows `SERVER_ADDRESS` / `SERVER_PORT` (inherited
environment, then `.env`, then project defaults); wildcard addresses are tested
via loopback. Refresh with `F5` after changing these settings.

Select a route with the arrows and press `x` or Enter. POST opens a field form
prepopulated from the model: select **Envoyer la requête** to send immediately, or
navigate with arrows and edit individual values with Enter. Miblo serializes JSON;
users never need to enter JSON syntax. Numbers and booleans retain their types,
nullable fields start as NULL, unique examples change for each prepared request,
and private fields are masked. The generated `id` is not included.

For PATCH, all fields start excluded. Edit or include only the fields to change;
Space toggles inclusion, and `n` toggles NULL for nullable fields. An empty PATCH
is refused. Foreign keys offer up to 20 existing rows fetched through bounded,
background GET requests to the related collection. Required references are
preselected when a row is available. No related row is ever created automatically;
if lookup fails or the table is empty, select a real ID manually or create a row
first. Optional references can remain NULL.

Custom JSON routes use the associated model as an **editable suggestion**, not as
an inferred contract. Without a declared model, the form starts empty. `+` adds a
text, integer, decimal or boolean field, Delete removes a field, and Escape backs
out or cancels without sending. Nested custom payloads and automatic inference of
handwritten Rust request types are not supported yet. These draft fields remain
in the request form and do not alter project definitions. `f` also uses this form
for free POST/PUT/PATCH requests, recognizing known CRUD paths.

A POST/PATCH/PUT request is sent only when **Envoyer la requête** is selected and
Enter is pressed. Bodyless DELETE requests retain their confirmation dialog.
Results show the HTTP code, elapsed time and formatted response. Responses are
limited to 64 KiB, requests time out after ten seconds, and redirects are not
followed. No write request runs automatically. Custom authentication headers
are not yet supported.

The code preview occupies six terminal rows and can be hidden with `v`; it is
hidden automatically below 30 rows. `e` opens the selected file regardless of
preview visibility. Stop and normal quit
terminate processes started by this workspace, including Cargo's API child.
Finish generation/check jobs and pending HTTP requests before quitting; the UI
blocks quit while those jobs are active to avoid interrupting file writes. SQLite setup is automatic;
configure MySQL from the `b` menu before starting its API.

Without an interactive terminal, bare `miblo` displays help instead of waiting
for input. Explicit commands remain available for scripts and automation.

## Models and database setup from the interface

Press `a` to add a model such as `User`. Select its fields and options (nullable,
unique, private), then confirm: Miblo generates the Rust model, SQL migration and
CRUD routes. Existing application code is preserved. Press `m` to add optional fields or relations to an existing model; Miblo generates an additive migration.

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

## Database schema and relations

Press **d** (or select the schema from **b**) to inspect the actual database:
tables, column types, nullability, primary keys (`PK`), foreign keys (`FK`),
outgoing links, incoming links and deletion rules. These are read from SQLite
PRAGMA metadata or MySQL information_schema; handwritten tables are included,
internal SQLite/SQLx tables are hidden, and no user records are queried.
Use the arrows to select a table, Tab to focus its details, then arrows or
PageUp/PageDown to scroll. **F5** refreshes the snapshot. **p** switches to the
models declared in `miblo.yaml`, which remain visible without a database connection.
The two sources are labeled explicitly: declaring a model does not mean its
migration is already applied. `miblo db schema` returns the actual schema as JSON.

Each generated table already has an automatic integer primary key named `id`.
To create a relation while adding a **new** model:

1. Create the referenced model first, for example `Group` with `name:string`.
2. Press **a** to create `User` and add its regular fields.
3. In the field menu, choose **Ajouter une relation (clé étrangère)**.
4. Select `Group`; Miblo suggests `group_id` and uses the matching integer type.
5. Choose whether the link is optional (`NULL`) or unique (one-to-one instead
   of many-to-one), then choose the deletion rule. **RESTRICT** is the default:
   deleting a referenced group is blocked. **CASCADE** deletes the linked rows;
   **SET NULL** keeps them with an empty link and requires an optional field.
6. Confirm generation. Miblo creates the field, an index, the SQL foreign key,
   the Rust types and the CRUD payloads. SQLite applies the migration automatically
   while the API is stopped; use **b** to apply it with MySQL.

A field can also be a relation from the initial type selector. Self references
are supported (for example optional `Folder.parent_id → Folder.id`). The same
choices can be scripted:

```sh
miblo add Group name:string --yes
miblo add User name:string 'group_id:int:ref=Group' --yes
miblo add Note body:text 'user_id:int?:ref=User:on_delete=set_null' --yes
```

References target another Miblo model's generated `id`. They are enforced by the
database, including create and PATCH requests; related records must already
exist. The CRUD exposes the foreign-key ID, without automatic joins or nested
objects. Changing keys on existing models, custom/composite primary keys, and
automatic many-to-many tables are not part of this version. Existing composite
keys and foreign keys can still be inspected in the actual schema.

**Older projects:** press **u**, choose the published pack, a local directory or
a custom archive URL, and confirm. This updates only the template pin for future
models and routes; no existing Rust files, tables or applied migrations are rewritten.
The current packs are version **0.6.0**, format **3**, and require CLI **0.9+**. The previous 0.3.0, 0.4.0 and 0.5.0 packs remain available. The
CLI refuses to silently generate a relation with an older pack. Existing custom
error handlers are preserved; if an older parent route reports a generic error
on a protected deletion, the database still blocks it. Update its error mapping
in your editor if you want a specific response.

```sh
miblo template update --project my-api
# Local development:
miblo template update --project my-api --template-dir ../miblo_templates/rust_sqlite_template_v4
```

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
https://github.com/NaSuRall/miblo_templates/releases/download/v0.6.0/rust-axum-mysql.tar.gz
https://github.com/NaSuRall/miblo_templates/releases/download/v0.6.0/rust-axum-sqlite.tar.gz
```

Each archive has a `.sha256` sidecar in its versioned GitHub release.
No local template folder is needed for the default remote command.
For template development, with both repositories checked out next to one another:

```sh
miblo new shop-api --database mysql --template-dir ../miblo_templates/rust_template_v4
```

Do not point this option at the legacy `rust_template/config.yaml`: the new
pack has a `manifest.json` and separate `project/` and `resource/` templates.
`MIBLO_TEMPLATE_DIR` can set a local development source once per shell session.

## Noninteractive creation

```sh
miblo new shop-api \
  --template-dir ../miblo_templates/rust_template_v4 \
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
Whitespace-only registry formatting and rustfmt reordering of module declarations are accepted. If registry content changes,
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

## Evolve existing models without deleting rows

Press **m** (or `:model` / the action palette), choose a model, then add fields or
relations with the arrows. Existing fields and the automatic primary key are
shown first. New fields are always **optional**: existing rows keep `NULL` until
you fill them through the generated PATCH routes. Relations can target any
existing model, including one created later or the same model.

Review the SQL before generating; PageUp/PageDown scroll long dialog details.
Miblo creates a new `ALTER TABLE` migration and updates the Rust model and CRUD
handlers. It never drops/rebuilds the table, disables foreign keys, or rewrites
old migrations. Unique fields and relation indexes are generated too.

```sh
miblo edit User 'nickname:string?:unique' 'group_id:int?:ref=Group:on_delete=set_null' --dry-run
miblo edit User 'nickname:string?:unique' 'group_id:int?:ref=Group:on_delete=set_null' --yes
miblo db migrate
cargo run
```

Stop the API before editing. Generation does **not** apply the migration, including
on SQLite: the orange BDD status signals that it is pending. Use **b → Appliquer
les migrations**, then **r** to rebuild/restart. The generated API also applies
pending migrations on startup. This works with both legacy **0.3.0** and current **0.6.0** remote packs;
no local template directory is required. Each model keeps its own pinned pack.

Miblo saves pristine generated resource sources in `.miblo/state.json` and uses
three-way merging to retain custom Rust changes. `rustfmt` must be installed
(`rustup component add rustfmt`); formatting can change. On overlapping edits or
invalid Rust syntax, generation aborts before changing any project files. Review
and reconcile the conflicting custom code before retrying. Projects from earlier
versions reconstruct their first baseline from their pinned template. Keep using
Miblo **0.8+** for projects with custom routes.

Replaced source/configuration/state files are copied to
`.miblo/backups/<migration-version>/`. These are code backups, **not database
backups**. If reverting before application, restore all backed-up files and remove
the corresponding unapplied migration together. Never remove an applied migration
or restore an old model independently of its database schema.

This editor currently supports **additions**. Renaming/removing columns, changing
existing types, making fields required and changing existing relationship rules
need a separate explicit migration; these operations are not generated by the
interface. It does not invent values or relationships for existing rows.

## Generated code for human maintenance

New APIs use small, ordinary Rust modules, formatted with rustfmt during
generation. Install the formatter with `rustup component add rustfmt` if needed.
Comments and the generated getting-started README are in French; identifiers
follow Rust conventions in English. There is no extra application framework.

```text
src/main.rs                  server startup
src/config.rs                environment configuration
src/database.rs              pool and migrations
src/lib.rs                   shared state, health route and route assembly
src/resources/user/
  mod.rs                     route definitions
  handlers.rs                HTTP extraction and responses
  model.rs                   User database/response structure
  requests.rs                CreateUser, UpdateUser, pagination and validation
  service.rs                 business rules and use cases
  repository.rs              SQL queries and transactions
```

A write follows `handlers → service → repository`. Add business rules in
`service.rs`, validation in `requests.rs` and query changes in `repository.rs`.
Handlers and services do not enumerate model fields, so adding a field leaves
these files byte-for-byte intact. Model/request/SQL changes use the existing
three-way merge and backups. Each resource includes request validation tests.

The generated README covers setup, the file map, customization examples, curl
requests, relations, migrations and testing. Applications run with Cargo without
requiring Miblo at runtime.

Updating templates with **u** affects future models. Existing models retain
their structure and their original template pin for subsequent edits; a project
may contain both layouts. Existing application files and migrations are not
rewritten. The format-1 folders and published archives remain available. An older
pack that lacks relations still requires a deliberate code/template upgrade to
add relations to that model.

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
# Fetches /v0.6.0/rust-axum-mysql.tar.gz and its .sha256 sidecar.

miblo new shop-api \
  --template-url https://templates.example.com/custom/api.tar.gz \
  --template-version 0.6.0 --yes
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

The CLI tests cover download/cache behavior, HTTP responses and confirmation,
compact dashboard layout, and actual SQLite migration status (pending and changed SQL).
To test the actual template output with MySQL installed (`mysql` and `mysqld`):

```sh
cargo build --locked
python3 scripts/smoke_mysql.py \
  --miblo target/debug/miblo \
  --templates ../miblo_templates/rust_template_v4
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
  --templates ../miblo_templates/rust_sqlite_template_v4 \
  --target-dir target/sqlite-smoke
```

It checks empty-project startup, automatic file creation, model additions,
migrations, CRUD, concurrent PATCH and persistence across restarts. `--remote`
replaces the local template option to verify the published SQLite pack.

A PTY smoke test exercises the real full-screen UI on Unix/macOS. It requires
`pyte` in a separate Python environment. It creates an isolated SQLite database
and uses a stub external editor. With `--api-target-dir` it compiles and runs a
real API and exercises CRUD through the interface (including cancellation and
HTTP 404), relation creation, actual/declared schema, and template updates. Without it, the process fixture intentionally does not answer HTTP
and must be reported as unresponsive:

```sh
python3 scripts/smoke_tui.py \
  --miblo target/debug/miblo \
  --database sqlite --templates ../miblo_templates/rust_sqlite_template_v4 \
  --api-target-dir /tmp/miblo-tui-api-target
```

The CLI is organized around `config`, `templates`, `generation`, `wizard`,
`cli`, `database`, `runtime` and `tui`. The production template pack stays in the separate
`miblo_templates` repository; it is not embedded in the CLI binary.

## Next steps

Authentication with protected routes and proper password handling, richer field
types, automatic many-to-many relations, OpenAPI, existing-schema evolution, then one Node/TypeScript
backend. These are not implemented by this release.

License: MIT.

## Créer une route depuis Miblo

**Une route liée à un modèle peut exécuter son SQL automatiquement.** Avec **g**,
choisissez le modèle puis l’action : lister, consulter par id, créer, modifier
ou supprimer. Miblo déduit la méthode HTTP et le formulaire de test. Les routes
réutilisent les handlers CRUD du modèle, leurs validations, les champs privés et
les requêtes SQL existantes. Aucune nouvelle table ni migration n’est nécessaire.

```sh
miblo route add members --resource User --action list --path /members --yes
miblo route add create_member --resource User --action create --path /members --yes
miblo route add member --resource User --action get --path '/members/{id}' --yes
miblo route add edit_member --resource User --action update --path '/members/{id}' --yes
miblo route add delete_member --resource User --action delete --path '/members/{id}' --yes
```

Le chemin peut différer de celui du modèle. Les actions sur une ligne exigent
un unique paramètre `{id}` ; lister et créer n’en acceptent pas. Le dossier de la
route contient `mod.rs` et `handlers.rs`, avec un alias explicite vers le handler
du modèle. Les changements du modèle sont donc partagés avec ces routes.
Les URL ne définissent pas de filtre ni d’authentification : « lister » renvoie
la liste paginée (`limit`, `offset`), même si le chemin s’appelle `/users/active`.

Le pack **0.6.0** est requis : **u → GitHub Releases** pour les anciens projets.
Le code existant et les modèles gardent leur architecture d’origine.

### Logique personnalisée

Choisissez **Logique personnalisée** pour créer une fonction métier indépendante.

Appuyez sur **g** ou choisissez **Créer une route de modèle / personnalisée** dans la palette.
Sélectionnez le modèle associé (ou une route indépendante), la méthode HTTP,
le chemin, le nom de la fonction et éventuellement un corps JSON. La confirmation
affiche les fichiers à créer. `Échap` annule sans écrire.

```sh
miblo route add active_users --method GET --path /users/active --resource User --yes
miblo route add activate_user --method POST --path '/users/{id}/activate' --resource User --json-body --yes
# --dry-run affiche les fichiers prévus sans écrire ; --offline réutilise le cache.
```

La route reçoit son propre dossier `src/resources/custom_routes/<nom>/` :
`mod.rs` (branchement HTTP), `handlers.rs` (extraction et réponse),
`requests.rs` (paramètres à typer), `service.rs` (fonction métier à compléter).
Les paramètres du chemin sont des chaînes, les filtres une table de chaînes,
et le corps JSON facultatif un `serde_json::Value`, faciles à remplacer par vos DTO.
Le pool SQL de l’API est transmis à la fonction métier.

La réponse initiale est **501 Not Implemented**. Remplacez le corps de la fonction
métier, puis redémarrez l’API. Sélectionnez la route et appuyez sur **e** pour ouvrir
sa fonction `service.rs` dans l’éditeur, ou sur **x** pour la tester avec le formulaire de champs
(suggestions du modèle associé, champs libres pour les routes sans contrat). Aucun changement de table ni migration n’est généré.
Les autres handlers, services et fichiers personnalisés restent intacts.

Les chemins utilisent `/segments/{parametres}` sans query string ni wildcard.
Les doublons de méthode/chemin, les collisions de paramètres et les conflits avec
le CRUD ou `/health` sont refusés avant écriture ; GET réserve aussi HEAD.
Les routes branchées manuellement hors de Miblo restent à vérifier par vos soins.
Les deux registres `src/resources/mod.rs` et `custom_routes/mod.rs` sont gérés
par Miblo : personnalisez les fichiers de chaque route plutôt que ces registres.

Pour un projet existant, utilisez **u → GitHub Releases** ou `miblo template update`
avant la première création de route. Le pack **0.6.0** propose aussi les actions CRUD automatiques ; les modèles
existants gardent leurs templates d’origine et leur code personnalisé. Les templates
restent téléchargés et mis en cache, sans être intégrés au binaire.
