use std::error::Error;
use serde_json::Value;

pub fn reader_json(input: Result<String, Box<dyn Error>>) {
    // Vérification si on a une erreur
    let raw = match input {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Erreur reçue : {}", e);
            return;
        }
    };

    // Convertir la String JSON en Value
    let json: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("JSON invalide : {}", e);
            return;
        }
    };

    let route = json["route"].as_array().cloned().unwrap_or_default();
    let models = json["models"].as_array().cloned().unwrap_or_default();
    let serveur = json["serveur"].as_array().cloned().unwrap_or_default();
    let database = json["database"].as_array().cloned().unwrap_or_default();

}