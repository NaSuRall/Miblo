use crate::writer::writer_routes;
use handlebars::Handlebars;
use serde_json::json;
use std::{error::Error, path::PathBuf};

use crate::cli::config::MibloConfig;

pub fn generate(project_path: &PathBuf, miblo_config: &MibloConfig) -> Result<(), Box<dyn Error>> {
    let mut hbs = Handlebars::new();

    let template_path = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("routes.rs.hbs");

    hbs.register_template_file("routes", template_path)
        .expect("Failed to read routes template file");

    let data = json!({
        "routes": miblo_config.routes.iter().map(|r| json!({
            "path": r.path,
            "method": r.method,
            "model_low": r.method.to_lowercase(),
            "model": capitalize(&r.model),

        })).collect::<Vec<_>>()
    });

    let res = hbs
        .render("routes", &data)
        .expect("Failed to read data from routes template");

    // lancer le writeer

    writer_routes::write_routes(project_path, res, miblo_config)
        .expect("failed to write routes from generator");

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}
