# Miblo

[![crates.io](https://img.shields.io/crates/v/miblo.svg)](https://crates.io/crates/miblo)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/NaSuRall/Miblo/blob/main/LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org/tools/install)

**Create a Rust API from your terminal.** Choose SQLite or MySQL, add models and
relations, generate CRUD routes, and try requests from the same interface.
Miblo produces an ordinary **Axum + SQLx** project that you can read, edit and run
without Miblo.

[Website](https://cli.miblo.fr/) · [English documentation](https://cli.miblo.fr/en/docs/)
· [Documentation française](https://cli.miblo.fr/docs/) · [Downloads](https://github.com/NaSuRall/Miblo/releases)
· [Source code](https://github.com/NaSuRall/Miblo)

Miblo is a public beta. It generates the foundation of an API; you supply the
business rules, authentication, permissions and deployment configuration.

## Install

Install [Rust](https://www.rust-lang.org/tools/install) **1.88 or later**, Cargo
and rustfmt, then:

```sh
rustup component add rustfmt
cargo install miblo --locked
miblo
```

Use a terminal at least **70 columns × 20 rows**. If `miblo` is not found, reopen
your terminal and check that Cargo's `bin` directory is on your `PATH`:
`~/.cargo/bin` on macOS/Linux, `%USERPROFILE%\.cargo\bin` on Windows.

[GitHub Releases](https://github.com/NaSuRall/Miblo/releases) also provide binaries
for **Windows x64** (`miblo.exe`), **Linux x64** and **macOS Intel / Apple Silicon**.
Extract the matching archive, verify its accompanying `.sha256` checksum, then
add the executable directory to your `PATH`. Linux binaries require glibc 2.35+.
The beta binaries are not signed or notarized.

Rust, Cargo and rustfmt are still needed with a prebuilt Miblo binary because
Miblo generates Rust projects. You also need native build tools: a C/C++ toolchain
on Linux, Command Line Tools on macOS, or Visual Studio Build Tools with
**Desktop development with C++** on Windows.

**SQLite needs no database server.** MySQL requires an accessible server.
SQLx CLI, Python and Docker are not required to use Miblo. Templates download on
demand from versioned GitHub Releases and stay in a local cache; they are not
embedded in the executable. No local template checkout is needed.

## Your first API

1. Run `miblo`, press **n**, name your project and choose **SQLite**.
2. Keep **GitHub Releases** as the template source and confirm creation.
3. Press **a** to add a model such as `User`. Choose fields and options with the
   arrows; Miblo creates its table, Rust types and CRUD routes.
4. Press **r** to compile and start the API. The status indicator turns green
   when its health route responds.
5. Select a route and press **x** to test it. POST requests open a prefilled
   field form: review the values and select **Send request**.

SQLite configuration and initial migrations are automatic. For MySQL, the
interface asks for connection settings. Generating the files is quick; the first
Rust compilation can take several minutes.

## What you can build

| In Miblo | Result |
| --- | --- |
| Models and fields | Rust types, SQL tables and list/get/create/update/delete routes |
| Relations | Foreign keys, indexes and deletion rules; each table has an automatic `id` |
| Add fields to an existing model | Optional fields and an additive migration; existing rows retain `NULL` |
| Routes linked to a model | Reuse that model's CRUD handlers and SQL at another URL |
| Custom routes | Separate HTTP handler and business function to complete in your editor |
| Database view | Tables, columns, primary keys and relationships, from the live database or project definition |
| HTTP tester | Typed fields, optional values and relation choices without writing JSON |
| API and database status | Health checks, pending migrations, errors and process logs |

Model additions and supported edits preserve custom code. Conflicting edits
stop generation before writing. Existing migrations are kept. Applying migrations
for an edited model is a separate action from generating its code.

Custom business routes initially return **501 Not Implemented** until you fill
in their service function. To get working CRUD immediately, choose a model action
such as **list**, **create** or **update**. A route name alone does not define SQL
filters or business rules.

## Terminal controls

| Key | Action |
| --- | --- |
| Arrows / `j` / `k` | Navigate |
| `Tab` / `Shift+Tab` | Switch panels |
| `Space` | Open all actions |
| `n` / `o` | Create / open an API project |
| `a` / `m` | Add / extend a model |
| `g` / `x` | Create / test a route |
| `b` / `d` | Database setup and migrations / schema |
| `r` / `s` | Start / stop the API |
| `e` / `v` | Open code in your editor / toggle the small preview |
| `Shift+L` | Choose English or French |
| `:` / `?` / `q` | Commands / help / quit |

The code preview is read-only. Set `VISUAL` or `EDITOR` to choose your editor,
for example `EDITOR="code --wait"`. API and Cargo processes run in the background
while their output appears in the log panel.

### English / Français

**Available since Miblo 0.9.1.**

Miblo starts in **English** by default. Press **Shift+L**, or open
**Space → Language / Langue**, choose **English** or **Français**, then press Enter.
You can also use `:language` or `:langue`.

The interface changes immediately and remembers your choice for the next launch.
Your project stays open and the API keeps running. Preferences belong to your
user account, outside your API project. Generated files, data and existing logs
are unchanged. Command names and `--help` stay in English; generated comments and
README files currently follow the French template packs.

For scripts, `MIBLO_LANGUAGE=en` or `MIBLO_LANGUAGE=fr` overrides the saved
preference. [Preference locations and configuration](https://github.com/NaSuRall/Miblo/blob/main/docs/REFERENCE.md#language--langue).

## Prefer commands?

The same generator works from scripts. This example creates a SQLite API using
remote templates, without any interactive prompts:

```sh
miblo new shop-api --database sqlite --yes
cd shop-api
miblo add Group name:string --yes
miblo add User name:string 'group_id:int?:ref=Group' --yes
miblo route add members --resource User --action list --path /members --yes
miblo db init
miblo run
```

`GET /members` reuses the User list operation. Other generated routes include
`GET /users`, `POST /users`, `GET /users/{id}`, `PATCH /users/{id}` and
`DELETE /users/{id}`. `GET /health` reports liveness.

Use `--dry-run` on generation commands to preview changes, and `--offline` to
reuse a cached template pack. Run `miblo --help` or `miblo <command> --help` for
options. Launch `miblo` inside a generated project to open it in the interface.

## Code you can maintain

Each model has its own directory:

```text
src/resources/user/
  mod.rs          route definitions
  handlers.rs     HTTP extraction and responses
  model.rs        database / response type
  requests.rs     request types and validation
  service.rs      business rules
  repository.rs   SQL queries
```

The generated API runs with Cargo independently of Miblo. Extend its services,
request types and queries in your editor. The generated README explains its
structure, setup, migrations and example requests.

Currently supported: Rust, Axum, SQLx, SQLite and MySQL. Composite primary keys,
automatic many-to-many tables, destructive model edits and OpenAPI generation
are not yet provided. See the [full reference](https://github.com/NaSuRall/Miblo/blob/main/docs/REFERENCE.md)
for supported operations, template pinning, merge behavior and current limits.

## Development

To try changes from this repository before the next release:

```sh
cargo install --path . --locked
```

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

CI checks Linux, macOS, Windows and the minimum Rust version, installs the
packaged crate, and tests generated SQLite/MySQL APIs. Terminal tests cover
creation, model changes, relations, routes, HTTP requests and both languages.
[Detailed checks](https://github.com/NaSuRall/Miblo/blob/main/docs/REFERENCE.md#development-and-checks)
· [Changelog](https://github.com/NaSuRall/Miblo/blob/main/CHANGELOG.md)
· [Report a bug](https://github.com/NaSuRall/Miblo/issues).

## License

[MIT](https://github.com/NaSuRall/Miblo/blob/main/LICENSE).
