use crate::engine::create_folder;
use crate::parser::config_reader;
use crate::parser::reader_yaml;
use crate::runtime;
use crate::services::generator_service;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
pub mod config;

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        template_dir: PathBuf,
    },
    Run {
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Parser)]
#[command(name = "miblo", about = "Génerate Api Rust")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

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
            let generator = generator_service::Generator::new(project_path, miblo_config);
            generator.generator_all(name)?;
        }
        Command::Run { name } => {
            runtime::start(&name.to_string())?;
        }
    }
    Ok(())
}
