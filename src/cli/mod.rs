use clap::{Parser, Subcommand};

use crate::generator;
use crate::runtime;

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
            println!("Lancement du generateur");
            let _ = generator::generator(name);
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
