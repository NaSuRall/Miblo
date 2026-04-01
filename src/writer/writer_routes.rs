use std::io::Write;
use std::env;
use std::fs::{OpenOptions};


pub fn write_routes(name: &str, routes: String) -> Result<() , Box<dyn std::error::Error>>{


    println!("{:?}", routes);


    let current_dir = env::current_dir()?;
    let project_path = current_dir.join(name);
    let route_dir = project_path.join("src/routes");

    let mod_file_path = route_dir.join("mod.rs");

    let mut mod_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(mod_file_path)?;

    mod_file.write_all(routes.as_bytes())?;    

    Ok(())
}
