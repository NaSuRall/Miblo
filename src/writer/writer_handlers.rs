use std::env;
use std::fs::File;
use std::path::PathBuf;
use serde_json::Value;
use std::io::Write;
use crate::cli::config::MibloConfig;
use crate::generator::generator_handler::generate_handler;



pub fn write_handlers(project_path: &PathBuf, miblo_config: &MibloConfig) -> Result<(), Box<dyn std::error::Error>>{

    // let current_dir = env::current_dir()?;
    //let project_path = current_dir.join(name);
    let handler_dir = project_path.join("src/handlers");

    let mod_file_path = handler_dir.join("mod.rs");
    let mut mod_file = File::create(&mod_file_path)?;

        writeln!(mod_file, "pub mod register;")?;
        writeln!(mod_file, "pub mod login;")?;

    let generated_handlers = generate_handler(miblo_config);
    
    for (handler_name, content) in generated_handlers {

        let file_name = format!("{}.rs", handler_name.to_lowercase());
        let file_path = handler_dir.join(&file_name);

        let mut file = File::create(file_path)?;

        file.write_all(content.as_bytes())?;

      
        writeln!(mod_file, "pub mod {};", handler_name.to_lowercase())?;
    }

    Ok(())
}
