use handlebars::Handlebars;
use serde_json::Value;
use std::env::current_dir;
use std::process::{Command};
use serde_json::{json};
pub fn generator(name: &str, models: &Vec<Value>)  {

    let mut handlebars = Handlebars::new();
    let current_dir = current_dir().expect("Impossible de lire le dossier");
    let project_path = current_dir.join(&name);

    


    // lancer la commande pour crée une migration 

    // Lancer la commande pour créer une migration
    let output = Command::new("sqlx")
        .args(["migrate", "add", "create_user"])
        .current_dir(&project_path) // important : exécuter dans le bon dossier
        .output()
        .expect("Impossible de lancer sqlx");

    let stdout = String::from_utf8(output.stdout)
        .expect("Impossible de lire le stdout");

    // "Creating migrations/20240414123045_create_user.sql"
    let migration_path = stdout
        .split_whitespace()
        .last()
        .unwrap();

    println!("DEBUG TEST {:?}", migration_path);
    println!("DEBUG MODELS : {:#?}", models);

    // Aller dans le dossier de la migration 
    // et ecrire les models etc
    

    let migration_dir = project_path.join(migration_path);

    
    handlebars.register_template_file("migration", "src/templates/rust/migration.rs.hbs")
        .expect("Impossible d'acceder au template migration");
    
    let data = json!({
        
    });

    handlebars.render("migration", &data).expect("Erreur render hbs migration");




}
