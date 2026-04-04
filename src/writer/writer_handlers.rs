use std::env;
use std::fs::File;
use serde_json::Value;
use std::io::Write;
use crate::generator::generator_handler::generate_handler;



pub fn write_handlers(name: &str, handler: &Vec<Value>) -> Result<(), Box<dyn std::error::Error>>{

    let current_dir = env::current_dir()?;
    let project_path = current_dir.join(name);
    let handler_dir = project_path.join("src/handlers");

    let mod_file_path = handler_dir.join("mod.rs");
    let mut mod_file = File::create(&mod_file_path)?;


    let generated_handlers = generate_handler(handler);
    
    for (handler_name, content) in generated_handlers {

        let file_name = format!("{}.rs", handler_name.to_lowercase());
        let file_path = handler_dir.join(&file_name);

        let mut file = File::create(file_path)?;

        file.write_all(content.as_bytes())?;

        writeln!(mod_file, "pub mod {};", handler_name.to_lowercase())?;
    }

    Ok(())
}
