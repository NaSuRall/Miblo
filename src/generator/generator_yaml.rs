use serde_json::Value;
use std::{error::Error, path::PathBuf};

use crate::{cli::config::MibloConfig};

pub fn reader_json(
    config_dir: PathBuf,
    json: Value,
) -> Result< MibloConfig , Box<dyn Error>> {
    let routes = serde_json::from_value(json["route"].clone())?;
    let models = json["models"].as_array().expect("");
    let server = json["server"].as_array().expect("");
    let database = json["database"].as_array().expect("");
    let auth = json["auth"].as_bool().expect("Auth n'a pas ete recuperer");
    let template_dir = json["template_dir"].as_str().unwrap().to_string();
    
    print!("{:?}", routes);
    // Mettre les donnes du json dans le Struct MibloConfig
    Ok(MibloConfig {
        models: models.clone(),
        routes,
        server: server.clone(),
        database: database.to_vec(),
        auth, 
        template_dir,
        config_dir
    })
}

