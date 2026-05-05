use std::path::PathBuf;

use crate::models::models_route::Route;
use crate::models::models_models::Model;
use crate::models::models_database::Database;
use crate::models::models_server::Server;

#[derive(Debug)]
pub struct MibloConfig {
    pub models: Vec<Model>,
    pub routes: Vec<Route>,
    pub server: Vec<Server>,
    pub database: Vec<Database>,
    pub auth: bool,
    pub template_dir: String,
    pub config_dir: PathBuf,

}
