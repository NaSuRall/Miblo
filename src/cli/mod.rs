use crate::generator::generator_routes;
use crate::generator::generator_template;
use crate::generator::generator_yaml;
use crate::parser;
use crate::runtime;
use crate::writer::writer_models;
use crate::writer::writer_routes;
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

            let current_dir = std::env::current_dir()?;
            let project_path = current_dir.join(&name);
            std::fs::create_dir_all(&project_path)?;

            let route_yaml_src = "src/templates/handlebars/rust/route.yaml";
            let route_yaml_dest = project_path.join("route.yaml");
            std::fs::copy(route_yaml_src, &route_yaml_dest)?;
            println!("{}", "route.yaml copié".yellow());


            // Lire le route.yaml
            let json_value = parser::reader_route(&name)?;
            println!("{}", "Lecture du fichier route.yaml....".yellow());
            let (routes, models, database, server) = generator_yaml::reader_json(json_value)?;
            let auth = true; // a mettre dans le yaml ca

            // Générer la structure de base
            generator_template::generator(&name, database, server, auth)?;
            println!("{}", "Structure de base générée".green());

            writer_models::write_model(&name, models)?;
            println!("{}", "Models créés".green());

            let code = generator_routes::generate_routes(&routes);
            writer_routes::write_routes(&name, code)?;
            println!("{}", "Routes créées".green());
            println!("{}", "Projet initialisé avec succès !".green());
        }

        Commands::Generate { name } => {
            // Regénère uniquement models et routes pour l'instant et puis la suite plus tard
            let json_value = parser::reader_route(&name)?;
            println!("{}", "Lecture du fichier route.yaml....".yellow());

            let (routes, models, _database, _server) = generator_yaml::reader_json(json_value)?;

            writer_models::write_model(&name, models)?;
            println!("{}", "Models mis à jour".green());

            let code = generator_routes::generate_routes(&routes);
            writer_routes::write_routes(&name, code)?;
            println!("{}", "Routes mises à jour".green());
        }

        Commands::Run { name } => {
            println!("Lancement du serveur : ");
            let _ = runtime::runtime(name);
        }
    }
    Ok(())
}
