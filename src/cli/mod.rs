use clap::{Parser, Subcommand};

use crate::generator;
use crate::parser;
use crate::runtime;
use colored::*;
use crate::generator::generator_yaml;

#[derive(Subcommand)]
enum Commands {
    Init { name: String },
    Run { name: String },
    ListeApp {},
}

#[derive(Parser)]
#[command(name = "miblo", about = "CLI pour générer des API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

pub fn lunch() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            let _ = generator::generator(&name);
            let json_route_yaml = parser::reader_route(&name);
            generator_yaml::reader_json(json_route_yaml);
        }
        Commands::Run { name } => {
            println!("Lancement du serveur : ");
            let _ = runtime::runtime(name);
        }

        Commands::ListeApp {} => {
            println!("Voici toutes vos applications : ");
        }
    }
}
