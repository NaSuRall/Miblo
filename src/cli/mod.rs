use std::env;
use std::path::PathBuf;

use crate::engine::fs::copy_dir_all;
use crate::generator::generator_routes;
use crate::generator::generator_handler;
use crate::generator::generator_template;
use crate::generator::generator_yaml;
use crate::generator::generator_sqlx;
use crate::engine::fs;
use crate::parser;
use crate::runtime;
use crate::writer::writer_handlers;
use crate::writer::writer_models;
use crate::writer::writer_routes;
use clap::{Parser, Subcommand};
use colored::*;



#[derive(Subcommand)]
enum Commands {
    Init { name: String },
    Run { name: String },
    Generate { name: String },
    Export { name: String, destination: PathBuf }
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

            // Copy le route.yaml du templates dans le projet ( en faire un fonction )
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
            

            // Générer le fichier des migrations
            
            generator_sqlx::generator(&name, &models);

            //

            writer_models::write_model(&name, &models)?;
            println!("{}", "Models créés".green());

            let code = generator_routes::generate_routes(&routes);
            writer_routes::write_routes(&name, code)?;
            

            // Pour chaque models, crée dans le /handler le (model.name).rs avec les CRUD de base
            // gerer le mod.rs avec tout les fichier aussi
            writer_handlers::write_handlers(&name, &models)?; 

            println!("{}", "Routes créées".green());
            println!("{}", "Projet initialisé avec succès !".green());
        }

        Commands::Generate { name } => {
            // Regénère uniquement models et routes pour l'instant et puis la suite plus tard
            let json_value = parser::reader_route(&name)?;
            println!("{}", "Lecture du fichier route.yaml....".yellow());

            let (routes, models, _database, _server) = generator_yaml::reader_json(json_value)?;

            writer_models::write_model(&name, &models)?;
            println!("{}", "Models mis à jour".green());

            let code = generator_routes::generate_routes(&routes);
            writer_routes::write_routes(&name, code)?;
            println!("{}", "Routes mises à jour".green());
        }

        Commands::Run { name } => {
            let _ = runtime::runtime(name);
        }

        Commands::Export { name, destination } => {
    
            // trouver le chemin de l'api generer
            let current_dir = env::current_dir()?;
            let project_path = current_dir.join(name);
            println!("PROJECT PATH : {:?}", project_path);
            // copier tout les fichier et dossier de l'api
            copy_dir_all(&project_path, &destination)?;
            println!("{}", "Project export OK".green());
            // coller tout les fichier vers le chemin demander
        }
    }
    Ok(())
}
