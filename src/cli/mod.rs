use std::env;
use std::path::PathBuf;
pub mod config;
use crate::engine::fs::copy_dir_all;
use crate::engine::global_fn::request_cretate_folder;
use crate::generator::generator_routes;
use crate::generator::generator_sql;
use crate::generator::generator_sqlx;
use crate::generator::generator_template;
use crate::generator::generator_yaml;
use crate::parser;
use crate::runtime;
use crate::writer::writer_handlers;
use crate::writer::writer_models;
use clap::{Parser, Subcommand};
use colored::*;

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        template_dir: PathBuf,
    },
    Run {
        name: String,
    },
    Generate {
        name: String,
    },
    Export {
        name: String,
        destination: PathBuf,
    },
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
        // NAME : Nom du projet
        // TEMPLATE_DIR : chemin vers template a utiliser
        Commands::Init { name, template_dir } => {
            // NOTE : RENDRE CELA VARIABLE
            let current_dir = std::env::current_dir()?;
            let project_path = current_dir.join(&name);
            request_cretate_folder(&project_path)?;

            // Vérification si le fichier de l'api
            // a bien ete crée ou pas
            if !std::fs::exists(&template_dir)? {
                return Err("Err de caca".into());
            }

            // Lecture du Fichier Route.yaml
            let json_value = parser::reader_route(&template_dir)?;
            let miblo_config = generator_yaml::reader_json(
                template_dir.parent().unwrap().to_path_buf(),
                json_value,
            )?;

            // GENERATEUR DE STRUCTURE
            // [ ROUTES , MODEL , HANDLERS , MIGRATION , SQL]
            generator_template::generator(&project_path, &name, &miblo_config)?;
            generator_sqlx::generator(&project_path, &miblo_config);
            writer_models::write_model(&project_path, &miblo_config)?;
            generator_sql::generator(&project_path, &miblo_config);
            generator_routes::generate_routes(&project_path, &miblo_config);
            writer_handlers::write_handlers(&project_path, &miblo_config)?;

            println!(
                "{}",
                "Structure de base générée [ MODELS, ROUTES, HANDLERS, ROUTE.YAML ]".green()
            );
            println!("{}", "Vous pouvez maintenant acceder a au fichier [route.yaml] pour y modifier les données !".yellow());
        }

        Commands::Generate { name } => {
            // Regénère uniquement models et routes pour l'instant et puis la suite plus tard
            // let json_value = parser::reader_route(&name)?;
            //println!("{}", "Lecture du fichier route.yaml....".yellow());

            // let (routes, models, _database, _server, _auth, _template_dir) = generator_yaml::reader_json(json_value)?;

            //writer_models::write_model(&name, &models)?;
            // println!("{}", "Models mis à jour".green());

            // generator_routes::generate_routes(&name , &routes);
            // writer_routes::write_routes(&name, code)?;
            // println!("{}", "Routes mises à jour".green());
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
