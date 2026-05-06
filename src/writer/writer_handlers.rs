use std::{error::Error, fs::File, path::PathBuf};
use std::io::Write;

pub fn writer(project_path: &PathBuf, handlers: Vec<(String, String)>) -> Result<(), Box<dyn Error>> {

        let handlers_dir = project_path.join("src/handlers");

        let mod_file_path = handlers_dir.join("mod.rs");
        let mut mod_file = File::create(&mod_file_path)?;

        writeln!(mod_file, "
            pub mod register;
            pub mod login;
        ")?;

        for (handler_name , content) in handlers {
            let file_name = format!("{}.rs", handler_name.to_lowercase());
            let file_path = handlers_dir.join(&file_name);

            let mut file = File::create(file_path)?;

            file.write_all(content.as_bytes())?;

            writeln!(mod_file, "pub mod {};", handler_name.to_lowercase())?;
        }


        Ok(())
}
