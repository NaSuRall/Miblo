use crate::cli::config::MibloConfig;
use crate::generator::generator_handler::generate_handler;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub fn write_handlers(
    project_path: &PathBuf,
    miblo_config: &MibloConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let handler_dir = project_path.join("src/handlers");

    if miblo_config.language == "rust" {
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
    } else {
        println!("No else for moment....");
        Ok(())
    }
}
