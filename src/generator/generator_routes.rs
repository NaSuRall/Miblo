use std::path::PathBuf;

use handlebars::Handlebars;
use serde::Deserialize;
use serde_json::json;

use crate::{cli::config::MibloConfig, writer::writer_routes};

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Route {
    pub method: String,
    pub model: String,
    pub path: String,
}

pub fn generate_routes(project_path: &PathBuf, miblo_config: &MibloConfig) {
    let mut hbs = Handlebars::new();

    let template_path = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("routes.rs.hbs");
    hbs.register_template_file("routes", template_path)
        .expect("Failed to register routes template");

    let data = json!({
        "routes": miblo_config.routes.iter().map(|r| json!({
            "path": r.path,
            "method": r.method.to_lowercase(),
            "model_low": r.model.to_lowercase(),
            "model": capitalize(&r.model)
        })).collect::<Vec<_>>()
    });
    let code = hbs.render("routes", &data).expect("Erreur render routes");

    let _ = writer_routes::write_routes(project_path, code, &miblo_config);
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}
