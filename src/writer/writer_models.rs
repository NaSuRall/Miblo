use std::io::Write;
use std::env;
use std::fs::{File, OpenOptions};
use serde_json::Value;
use crate::generator::generator_models::generate_model;

pub fn write_model(name: &str, models: Vec<Value>) ->  Result<(), Box<dyn std::error::Error>>{

    let current_dir = env::current_dir()?;
    let project_path = current_dir.join(name);
    let model_dir = project_path.join("src/models");

    let mod_file_path = model_dir.join("mod.rs");
    let mut mod_file = File::create(&mod_file_path)?;

    // ouvrir mod.rs
    //let mut mod_file = OpenOptions::new()
    //    .create(true)
    //    .append(true)
    //    .open(mod_file_path)?;

    let generated_models = generate_model(&models);

    for (model_name, content) in generated_models {

        let file_name = format!("{}.rs", model_name.to_lowercase());
        let file_path = model_dir.join(&file_name);

        let mut file = File::create(file_path)?;

        // écrit la struct générée
        file.write_all(content.as_bytes())?;

        // ajoute dans mod.rs
        writeln!(mod_file, "pub mod {};", model_name.to_lowercase())?;
    }
    Ok(())
}
