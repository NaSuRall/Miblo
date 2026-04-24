use std::io::Write;
use std::fs::{OpenOptions};
use std::path::PathBuf;


pub fn write_routes(project_path: &PathBuf, routes: String) -> Result<() , Box<dyn std::error::Error>>{

    let route_dir = project_path.join("src/routes");

    let mod_file_path = route_dir.join("mod.rs");

    let mut mod_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mod_file_path)?;

    mod_file.write_all(routes.as_bytes())?;    

    Ok(())
}
