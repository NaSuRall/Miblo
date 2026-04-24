use handlebars::Handlebars;
use std::path::PathBuf;
use std::process::{Command};
use crate::cli::config::MibloConfig;
use crate::engine::global_fn::map_type_sql;
use crate::writer;
use crate::engine::model_template::send_model_handelbars;


pub fn generator(project_path: &PathBuf, miblo_config: &MibloConfig)  {

    let mut handlebars = Handlebars::new();
    


    let template_path = miblo_config.config_dir.join(&miblo_config.template_dir).join("migration.sql.hbs");
    println!("CACAACACA {:?}", template_path);
    let _ = handlebars.register_template_file("migration", &template_path);


    let output = Command::new("sqlx")
        .args(["migrate", "add", "create_user"])
        .current_dir(project_path)
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
    
    let results = send_model_handelbars("migration",Some(map_type_sql), &miblo_config.models, &handlebars);


    // Ouvrir le Fichier migration_dir
    // lancer le writer ici
    writer::writer_migration::write_migration(migration_dir, results);

    
}
