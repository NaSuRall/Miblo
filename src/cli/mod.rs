use clap::{Parser, Subcommand};

use crate::generator;

#[derive(Subcommand)]
enum Commands {
    Init { name: String },
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

        Commands::ListeApp {} => {
            println!("Voici toutes vos applications : ");
        }
    }
}
