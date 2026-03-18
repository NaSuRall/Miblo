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

pub fn lunch()-> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            // generer le template
            let _ = generator::generator(&name);
            // lire le fichier route.yaml et resortir en json
            let json_value = parser::reader_route(&name)?;
            // recuperer le json le lire et le trier
            let (routes, models, database, server) = generator_yaml::reader_json(json_value)?;

            println!("Routes: {:?}", routes);
            println!("Models: {:?}", models);
            println!("Database: {:?}", database);
            println!("Server: {:?}", server);
            
            // generer les models, les routes, la conexion bdd et la database grace
            // au generator_yaml;

        }
        Commands::Run { name } => {
            println!("Lancement du serveur : ");
            let _ = runtime::runtime(name);
        }

        Commands::ListeApp {} => {
            println!("Voici toutes vos applications : ");
        }
    }
    Ok(())
}
