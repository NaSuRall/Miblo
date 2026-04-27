use crate::cli::config::MibloConfig;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub fn write_routes(
    project_path: &PathBuf,
    routes: String,
    miblo_config: &MibloConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let route_dir = project_path.join("src/routes");

    if miblo_config.language == "rust" {
        let mod_file_path = route_dir.join("mod.rs");

        let mut mod_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(mod_file_path)?;

        mod_file.write_all(routes.as_bytes())?;

        Ok(())
    } else {
        println!("No else for moment....");
        Ok(())
    }
}
