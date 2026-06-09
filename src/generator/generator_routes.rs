//! Axum router generator.
//!
//! Produces `src/routes/mod.rs` by rendering `routes.rs.hbs` with the full
//! list of routes from the config, including capitalized model names for handler imports.

use crate::writer::writer_routes;
use handlebars::Handlebars;
use serde_json::json;
use std::{error::Error, path::PathBuf};

use crate::cli::config::MibloConfig;

/// Generate the Axum router module at `<project_path>/src/routes/mod.rs`.
///
/// # Errors
///
/// Returns an error if the template cannot be loaded or if writing the output file fails.
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
            "model_low": r.model.to_lowercase(),
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

/// Capitalise the first character of `s`, leaving the rest unchanged.
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}
