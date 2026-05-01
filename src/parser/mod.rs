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

pub fn verify_config(json_value: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let models = json_value["models"].as_array().ok_or("models must be an array")?;
    let routes = json_value["route"].as_array().ok_or("routes must be an array")?;

    for model in models {
        let model_name = model["name"].as_str().ok_or("model must have a name")?;
        let methods = model["method"].as_array().ok_or("model method must be an array")?;

        for method in methods {
            let method_str = method.as_str().ok_or("method must be a string")?;
            let mut found = false;

            for route in routes {
                let route_model = route["model"].as_str().unwrap_or("");
                let route_method = route["method"].as_str().unwrap_or("");
                if route_model == model_name && route_method == method_str {
                    found = true;
                }
            }

            if !found {
                return Err(format!(
                    "La route {} est manquante pour le modèle {}",
                    method_str.to_uppercase(), model_name
                ).into());
            }
        }
    }

    Ok(json_value.clone())
}
