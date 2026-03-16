use serde_json;
use serde_yaml;
use std::env;
use std::path::PathBuf;

pub fn reader_route(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let current_dir: PathBuf = env::current_dir()?;

    // Aller dans l'api
    let project_path: PathBuf = current_dir.join(name);
    println!("project_path : {:?}", project_path);

    // Lire le fichier route.yaml
    let route_file = project_path.join("route.yaml");
    let reader = std::fs::File::open(route_file)?;

    // Récupérer les données YAML
    let data: serde_yaml::Value = serde_yaml::from_reader(reader)?;

    // Convertir en JSON (string)
    let json = serde_json::to_string_pretty(&data)?; // pretty pour lisibilité

    Ok(json)
}