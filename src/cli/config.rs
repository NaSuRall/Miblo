use std::{path::PathBuf};

use serde_json::Value;

use crate::generator::generator_routes::Route;

#[derive(Debug)]
pub struct MibloConfig {
    pub models: Vec<Value>,
    pub routes: Vec<Route>,
    pub server: Vec<Value>,
    pub database: Vec<Value>,
    pub auth: bool,
    pub template_dir: String,
    pub config_dir: PathBuf,
    pub language: String
} 

/*
impl MibloConfig {
    pub fn caca() {
            
    }

    pub fn new() -> Self {
        MibloConfig { 
            auth: true, 
            template_dir: "TA MERE".to_string() 
        }
    }
} 


pub fn test(){
    let config = MibloConfig::new();
    let bb = config.auth;
}
*/
