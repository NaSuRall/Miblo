use crate::generator::generator_routes;
use crate::generator::generator_template;
use crate::generator::generator_yaml;
use crate::parser;
use crate::runtime;
use crate::writer::writer_models;
use clap::{Parser, Subcommand};
use colored::*;
#[derive(Subcommand)]
enum Commands {
    Init { name: String },
    Run { name: String },
    Generate { name: String },
}

#[derive(Parser)]
#[command(name = "miblo", about = "CLI pour générer des API")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

pub fn lunch() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => {
            // generer le template
            let _ = generator_template::generator(&name);
            println!("{}", "Template generer".green());
        }

        Commands::Generate { name } => {
            let json_value = parser::reader_route(&name)?;
            println!("{}", "Lecture du fichier route.yaml....".yellow());

            let (routes, models, database, server) = generator_yaml::reader_json(json_value)?;
            println!("{}", "Création des models de l'api....".yellow());

            let _ = writer_models::write_model(&name, models);
            println!("{}", "Model Crée avec sucess".green());

            
            let code = generator_routes::generate_routes(&routes);

            println!("CODE ROUTE :  {:?}", code);

            println!("{:?}", server);
            println!("{:?}", database);
        }

        Commands::Run { name } => {
            println!("Lancement du serveur : ");
            let _ = runtime::runtime(name);
        }
    }
    Ok(())
}
