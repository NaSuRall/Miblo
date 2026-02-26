use clap::{Parser, Subcommand};

use crate::generator;

#[derive(Subcommand)]
enum Commands {
    CreateApp { name: String },
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
        Commands::CreateApp { name } => {
            println!("Création de l'API {}", name);
            generator::generator(name);
        }
        Commands::ListeApp {} => {
            println!("Voici toutes vos applications : ");
        }
    }
}
