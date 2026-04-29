use serde_json::Value;
use std::path::PathBuf;

pub fn reader_route(template_config: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {

    let reader = std::fs::File::open(template_config)?;
    // Récupérer les données YAML
    let data: serde_yaml::Value = serde_yaml::from_reader(reader)?;
    // Convertir en JSON (string)
    let json_value: Value = serde_json::to_value(&data)?;
    // Retourner le json
    Ok(json_value)
}
