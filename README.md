# Miblo

A CLI code generator that scaffolds production-ready **Axum + SQLx** REST APIs from a simple YAML configuration file and Handlebars templates.

## Features

- Generates a complete Rust project structure from a single `config.yaml`
- Produces models, handlers, routes, SQL queries and SQLx migrations
- Optional JWT authentication scaffolding
- Runs the generated project directly (`sqlx migrate run` + `cargo run`)

## Prerequisites

- Rust toolchain (stable)
- `sqlx-cli` installed: `cargo install sqlx-cli`
- A running PostgreSQL (or SQLite) instance

## Installation

```bash
cargo install miblo_cli_v2
```

Or build from source:

```bash
git clone https://github.com/NaSuRall/Miblo
cd Miblo
cargo install --path .
```

## Quick Start

**1. Clone the official templates:**

```bash
git clone https://github.com/NaSuRall/miblo_templates
```

The template repository has the following structure:

```
miblo_templates/
└── rust_template/
    ├── config.yaml          ← point --template-dir here
    └── handlebars/
        ├── .env.hbs
        ├── Cargo.toml.hbs
        ├── main.rs.hbs
        ├── config.rs.hbs
        ├── model.rs.hbs
        ├── handlers.rs.hbs
        ├── routes.rs.hbs
        ├── migration.sql.hbs
        ├── login.rs.hbs
        ├── register.rs.hbs
        ├── claim.rs.hbs
        ├── login_model.rs.hbs
        ├── register_model.rs.hbs
        └── sql/
            ├── get.sql.hbs
            ├── post.sql.hbs
            ├── patch.sql.hbs
            └── delete.sql.hbs
```

**2. Edit `config.yaml` to describe your project** (see [config.yaml Reference](#configyaml-reference) below).

**3. Scaffold the project:**

```bash
miblo init --name my_api --template-dir ./miblo_templates/rust_template/config.yaml
```

**4. Start the server:**

```bash
miblo run --name my_api
```

## config.yaml Reference

```yaml
language: rust

template_dir: "handlebars"  # folder name relative to config.yaml

server:
  - port: 8080
    address: "0.0.0.0"

database:
  - db_connexion: mysql
    db_host: localhost
    db_port: 3306
    db_database: my_db
    db_username: user
    db_password: secret

auth: false  # set to true to generate JWT login/register endpoints

models:
  - name: User
    method: [get, post, patch, delete]
    fields:
      - name: id
        type: Int
        primary_key: true
        auto_increment: true
      - name: email
        type: String
        not_null: true
        unique: true
      - name: username
        type: String
        not_null: true

routes:
  - method: get
    model: User
    path: /users
  - method: post
    model: User
    path: /users
  - method: patch
    model: User
    path: /users/{user_id}
  - method: delete
    model: User
    path: /users/{user_id}
```

### Field types

| Miblo type | Rust type | SQL type        |
|------------|-----------|-----------------|
| `String`   | `String`  | `VARCHAR(255)`  |
| `Int`      | `i32`     | `INTEGER`       |
| `Binary`   | `u8`      | `BINARY(16)`    |

### Field constraints

| Key              | Type   | Description                          |
|------------------|--------|--------------------------------------|
| `primary_key`    | `bool` | Marks the column as primary key      |
| `auto_increment` | `bool` | Adds `AUTO_INCREMENT` / `SERIAL`     |
| `not_null`       | `bool` | Adds a `NOT NULL` constraint         |
| `unique`         | `bool` | Adds a `UNIQUE` constraint           |

## Generated Project Structure

```
my_api/
├── Cargo.toml
├── .env
└── src/
    ├── main.rs
    ├── config.rs
    ├── models/
    │   ├── mod.rs
    │   └── user.rs
    ├── handlers/
    │   ├── mod.rs
    │   └── user.rs
    ├── routes/
    │   └── mod.rs
    └── sql/
        └── user/
            ├── get.sql
            ├── post.sql
            ├── patch.sql
            └── delete.sql
```

When `auth: true`, the following are also generated:

```
src/
├── handlers/login.rs
├── handlers/register.rs
├── models/claim.rs
├── models/login.rs
└── models/register.rs
```

## CLI Reference

```
miblo init --name <NAME> --template-dir <PATH>
```

| Flag             | Description                                        |
|------------------|----------------------------------------------------|
| `--name`         | Name of the project (also used as directory name)  |
| `--template-dir` | Path to the `config.yaml` inside your template dir |

```
miblo run --name <NAME>
```

| Flag     | Description                            |
|----------|----------------------------------------|
| `--name` | Name of the project directory to start |

Runs `sqlx migrate run` then `cargo run` inside the generated project.

## License

MIT
