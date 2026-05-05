use crate::cli::config::MibloConfig;
use std::fs::OpenOptions;
use std::io::Write;
use std::{error::Error, path::Path};

pub fn write_routes(
    project_path: &Path,
    routes: String,
    _miblo_config: &MibloConfig,
) -> Result<(), Box<dyn Error>> {
    let route_dir = project_path.join("src/routes");

    let mod_file_path = route_dir.join("mod.rs");

    let mut mod_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mod_file_path)?;

    mod_file.write_all(routes.as_bytes())?;

    Ok(())
}
