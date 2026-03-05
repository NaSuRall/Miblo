use serde_yaml;
use std::env;
use std::path::PathBuf;

// faire retourner en JSON les valleur de ROUTE.yaml
pub fn reader_route(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir: PathBuf = env::current_dir()?;

    // Aller dans l'api
    let project_path: PathBuf = current_dir.join(&name);
    println!("project_path : {:?}", project_path);
    // lire le fichier routes.serde_yaml
    let route_file = project_path.join("route.yaml");
    let reader = std::fs::File::open(route_file)?;

    // Récuperer les données
    let data: serde_yaml::Value = serde_yaml::from_reader(reader)?;
    println!("Data route.yaml : {:#?} ", data);
    // Retourner en Json 

    // etc...
    Ok(())
}
