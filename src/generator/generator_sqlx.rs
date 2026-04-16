use handlebars::Handlebars;
use serde_json::Value;
use std::env::current_dir;
use std::process::{Command};
use serde_json::{json};


pub fn generator(name: &str, models: &Vec<Value>)  {

    let mut handlebars = Handlebars::new();
    let current_dir = current_dir().expect("Impossible de lire le dossier");
    let project_path = current_dir.join(name);
    let mut results = Vec::new();

    handlebars.register_template_file("migration", "src/templates/handlebars/rust/migration.sql.hbs")
    .expect("Failed to register template file for handlebars");
    
    let output = Command::new("sqlx")
        .args(["migrate", "add", "create_user"])
        .current_dir(&project_path) // important : exécuter dans le bon dossier
        .output()
        .expect("Impossible de lancer sqlx");

    let stdout = String::from_utf8(output.stdout)
        .expect("Impossible de lire le stdout");


    let migration_path = stdout
        .split_whitespace()
        .last()
        .unwrap();

    let migration_dir = project_path.join(migration_path);
    


    for model in models {
      let name = model["name"].as_str().expect("Model name is not a string");
      let fields = model["fields"].as_array().expect("Field is not an array");
      let mut field_list = Vec::new();
    
        for field in fields {
            let field_name = field["name"].as_str().expect("field type");

            field_list.push(json!({
                "name": field_name,
            }))
        }
        
        let data = json!({
            "model_name": name,
            "fields": field_list
        });
        
        let rendered = handlebars
            .render("migration", &data)
            .expect("Erreur render template");

        results.push((name.to_string(), rendered));

    }

    // Ouvrir le Fichier migration_dir
    // lancer le writer ici


}
