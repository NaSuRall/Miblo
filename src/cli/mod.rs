//! Command-line interface definitions and main dispatch logic.
//!
//! This module owns the Clap [`Cli`] / [`Command`] types and the [`run`] entry-point
//! that is called from `main`.

use crate::engine::create_folder;
/*
use crate::generator::generator_handlers;
use crate::generator::generator_models;
use crate::generator::generator_routes;
use crate::generator::generator_sql;
use crate::generator::generator_sqlx;
use crate::generator::generator_tempalte;
*/
use crate::parser::config_reader;
use crate::parser::reader_yaml;
use crate::runtime;
use crate::services::generator_service;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Shared configuration types used across CLI and generators.
pub mod config;

/// Available sub-commands for the `miblo` binary.
#[derive(Subcommand)]
enum Command {
    /// Scaffold a new Axum + SQLx project from a template directory.
    Init {
        /// Name of the project to create (also used as the output directory name).
        #[arg(short, long)]
        name: String,
        /// Path to the `config.yaml` file inside your template directory.
        #[arg(short, long)]
        template_dir: PathBuf,
    },
    /// Start the generated project with `sqlx migrate run` then `cargo run`.
    Run {
        /// Name of the project directory to run.
        #[arg(short, long)]
        name: String,
    },
}

/// Top-level CLI struct parsed by Clap.
#[derive(Parser)]
#[command(name = "miblo", about = "Génerate Api Rust")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Parse CLI arguments and dispatch to the appropriate sub-command.
///
/// Returns `Ok(())` on success or a boxed error describing what went wrong.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Init { name, template_dir } => {
            // CREATION DU FICHIER
            let current_dir = std::env::current_dir().expect("Failed to get current dir !");
            let project_path = current_dir.join(name);
            create_folder::request_create_folder(&project_path)?;
            if !std::fs::exists(template_dir).expect("Folder not find") {
                return Err("Folder not find".into());
            }
            // LECTURE DU CONFIG.YAML
            let config_value =
                config_reader::reader(template_dir).expect("Failed to read your config template");
            let miblo_config =
                reader_yaml::reader(template_dir.parent().unwrap().to_path_buf(), config_value)?;

            // GENERATEUR CODE
            //

            let generator = generator_service::Generator::new(project_path, miblo_config);
            generator.generator_all(name)?;
            /*
            generator_tempalte::template(&project_path, name, &miblo_config)?;
            generator_sqlx::generate(&project_path, &miblo_config)?;
            generator_models::generate(&project_path, &miblo_config)?;
            generator_sql::generate(&project_path, &miblo_config)?;
            generator_routes::generate(&project_path, &miblo_config)?;
            generator_handlers::generate(&project_path, &miblo_config)?;
            */
        }
        Command::Run { name } => {
            runtime::start(&name.to_string())?;
        }
    }
    Ok(())
}
