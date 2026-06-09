//! YAML value → [`MibloConfig`] hydration.

use std::error::Error;
use serde_yaml::Value;
use std::path::PathBuf;
use crate::cli::config::MibloConfig;

use crate::models::models_models::Model;
use crate::models::models_database::Database;
use crate::models::models_route::Route;
use crate::models::models_server::Server;

/// Deserialize a raw [`serde_yaml::Value`] (produced by [`crate::parser::config_reader::reader`])
/// into a fully-typed [`MibloConfig`].
///
/// # Arguments
///
/// * `config_dir` – directory that contains `config.yaml` (stored verbatim in the returned config).
/// * `yaml` – the parsed YAML tree.
///
/// # Errors
///
/// Returns an error if any required key (`routes`, `models`, `server`, `database`, `auth`,
/// `template_dir`) is missing or has an unexpected type.
pub fn reader(config_dir: PathBuf, yaml: Value) -> Result<MibloConfig, Box<dyn Error>> {

    let routes: Vec<Route> = serde_yaml::from_value(yaml["routes"].clone()).expect("Failed to read routes value form yaml");
    let models: Vec<Model> = serde_yaml::from_value(yaml["models"].clone())?;
    let server: Vec<Server> = serde_yaml::from_value(yaml["server"].clone())?;
    let database: Vec<Database> = serde_yaml::from_value(yaml["database"].clone())?;
    let auth = yaml["auth"].as_bool().ok_or("failed to read auth from yaml")?;
    let template_dir = yaml["template_dir"].as_str().unwrap().to_string();

    Ok(
        MibloConfig {
            models,
            routes,
            server,
            database,
            auth,
            template_dir,
            config_dir,
        }
    )
}
