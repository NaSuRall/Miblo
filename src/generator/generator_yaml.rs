use std::error::Error;
use serde_json::Value;
use crate::generator::generator_models;
use crate::generator::generator_routes;
use crate::generator::generator_server;

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
    let server = json["serveur"].as_array().cloned().unwrap_or_default();
    let database = json["database"].as_array().cloned().unwrap_or_default();

    // donner les valeur pour les differente fonction dans les fichiers
    generator_routes::generate_routes(&route);
    generator_models::generate_model(&models);
    generator_server::generate_server(&server);

}