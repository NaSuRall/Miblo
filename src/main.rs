//! # Miblo CLI
//!
//! `miblo_cli_v2` is a code generator that scaffolds a complete [Axum](https://github.com/tokio-rs/axum) + [SQLx](https://github.com/launchbadge/sqlx) REST API from a YAML configuration file and Handlebars templates.
//!
//! ## Quick start
//!
//! ```bash
//! # Initialise a new API project
//! miblo init --name my_api --template-dir ./my_template/config.yaml
//!
//! # Start the generated API
//! miblo run --name my_api
//! ```
//!
//! ## How it works
//!
//! 1. You write a `config.yaml` that describes your models, routes, database, and server.
//! 2. `miblo init` reads that config, renders Handlebars templates, and writes a ready-to-compile Axum project.
//! 3. `miblo run` runs `sqlx migrate run` then `cargo run` inside the generated project.
//!
//! ## Modules
//!
//! | Module | Role |
//! |--------|------|
//! | [`cli`] | Clap CLI definitions and entry-point |
//! | [`engine`] | Handlebars rendering helpers and type mapping |
//! | [`generator`] | Per-artefact generators (models, routes, SQL, …) |
//! | [`models`] | Deserialization structs mirroring the YAML schema |
//! | [`parser`] | YAML reading and config hydration |
//! | [`runtime`] | Running the generated project via `cargo run` |
//! | [`services`] | Orchestration layer that wires generators together |
//! | [`writer`] | File-system writers for every generated artefact |

use colored::Colorize;

pub mod cli;
pub mod engine;
pub mod generator;
pub mod models;
pub mod parser;
pub mod runtime;
pub mod writer;
pub mod services;

fn main() {
    let cli = cli::run();

    match cli {
        Ok(_) => println!("{}", "Your API has been successfully generated".cyan()),
        Err(e) => println!("error : {e:?}"),
    };
}
