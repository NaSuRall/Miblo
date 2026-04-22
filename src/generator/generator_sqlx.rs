use handlebars::Handlebars;
use serde_json::Value;
use std::env::current_dir;
use std::process::{Command};
use crate::engine::global_fn::map_type_sql;
use crate::writer;
use crate::engine::model_template::send_model_handelbars;


pub fn generator(name: &str, models: &Vec<Value>)  {

    let mut handlebars = Handlebars::new();
    let current_dir = current_dir().expect("Impossible de lire le dossier");
    let project_path = current_dir.join(name);
    //let mut results = Vec::new();

    handlebars.register_template_file("migration", "src/templates/handlebars/rust/migration.sql.hbs")
    .expect("Failed to register template file for handlebars");
    
    let output = Command::new("sqlx")
        .args(["migrate", "add", "create_user"])
        .current_dir(&project_path) // important : exécuter dans le bon dossier
        .output()
        .expect("Impossible de lancer sqlx");

    let stdout = String::from_utf8(output.stdout)
        .expect("Impossible de lire le stdout");


    // recupere le nom du fichier de migration
    let migration_path = stdout
        .split_whitespace()
        .last()
        .expect("nom du fichier de migration non trouver");

    let migration_dir = project_path.join(migration_path);
    
    let results = send_model_handelbars("migration",Some(map_type_sql), models, &handlebars);


    // Ouvrir le Fichier migration_dir
    // lancer le writer ici
    writer::writer_migration::write_migration(migration_dir, results);

    
}
