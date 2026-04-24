use std::io::Write;
use std::fs::{File};
use std::path::PathBuf;
use crate::cli::config::MibloConfig;
use crate::generator::generator_models::generate_model;

pub fn write_model(project_path: &PathBuf, miblo_config: &MibloConfig) ->  Result<(), Box<dyn std::error::Error>>{

    let model_dir = project_path.join("src/models");
    let mod_file_path = model_dir.join("mod.rs");
    let mut mod_file = File::create(&mod_file_path)?;

    writeln!(mod_file, "
        pub mod claim;
        pub mod register;
        pub mod login;

        pub use claim::Claims;
        pub use login::AuthUser;
        pub use login::LoginRequest;
        pub use register::RegisterUser;
                
        // Here model auto : 
    ")?;

    let generated_models = generate_model(miblo_config);

    for (model_name, content) in generated_models {

        let file_name = format!("{}.rs", model_name.to_lowercase());
        let file_path = model_dir.join(&file_name);

        let mut file = File::create(file_path)?;
    
        let model_name_up = format!("{}{}", &model_name[..1].to_uppercase(), &model_name[1..]);
        // écrit la struct générée
        file.write_all(content.as_bytes())?;

        // ajoute dans mod.rs
        writeln!(mod_file, "pub mod {};", model_name.to_lowercase())?;
        writeln!(mod_file, "pub use {}::{};", model_name.to_lowercase(), model_name_up)?;
       //  writeln!(mod_file, "pub use {}::Delete{};", model_name.to_lowercase(), model_name_up)?;

    }
    Ok(())
}
