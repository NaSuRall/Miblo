//! Hydrated configuration passed to every generator.

use std::path::PathBuf;

use crate::models::models_route::Route;
use crate::models::models_models::Model;
use crate::models::models_database::Database;
use crate::models::models_server::Server;

/// Fully-parsed project configuration, built from `config.yaml` by [`crate::parser::reader_yaml`].
///
/// Every generator receives a reference to this struct so it can read the models,
/// routes, database settings, and server settings in one place.
#[derive(Debug)]
pub struct MibloConfig {
    /// Domain models declared in `config.yaml` (maps to `models:` key).
    pub models: Vec<Model>,
    /// HTTP routes declared in `config.yaml` (maps to `routes:` key).
    pub routes: Vec<Route>,
    /// Server binding settings (port, address). Currently only the first entry is used.
    pub server: Vec<Server>,
    /// Database connection settings. Currently only the first entry is used.
    pub database: Vec<Database>,
    /// Whether to generate JWT authentication boilerplate.
    pub auth: bool,
    /// Relative path (inside `config_dir`) to the Handlebars template folder.
    pub template_dir: String,
    /// Absolute path to the directory that contains `config.yaml`.
    pub config_dir: PathBuf,
}
