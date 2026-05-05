use std::error::Error;
use std::path::PathBuf;
use crate::engine::handlebars_model::RenderedModel;
use std::fs::File;
use std::io::Write;

pub fn writer_model(project_path: &PathBuf, models: Vec<RenderedModel>) -> Result<(), Box<dyn Error>>{
    let model_dir = project_path.join("src/models");
    let mod_file_path = model_dir.join("mod.rs");
    let mut mod_file = File::create(&mod_file_path)?;

    // NOTE: A CHANGER (AUTH)
    writeln!(mod_file,
    "
    pub mod claim;
    pub mod register;
    pub mod login;

    pub use claim::Claims;
    pub use login::AuthUser;
    pub use login::LoginRequest;
    pub use register::RegisterUser;
    "
    )?;

    for model in models {
        let file_name = format!("{}.rs", model.name.to_lowercase());
        let file_path = model_dir.join(&file_name);

        let mut file = File::create(file_path).expect("Filed to create file for models");

        let model_name_up = model.name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string() + &model.name[1..])
            .unwrap_or(model.name.clone());

        file.write_all(model.content.as_bytes())?;

        writeln!(mod_file,"pub mod {};", model.name.to_lowercase())?;
        writeln!(mod_file, "pub use {}::{};", model.name.to_lowercase(), model_name_up)?;
        writeln!(mod_file, "pub use {}::{}Option;", model_name_up.to_lowercase(), model_name_up)?;

    }
    Ok(())
}
