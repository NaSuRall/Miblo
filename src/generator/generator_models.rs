//! Model struct generator.
//!
//! Produces one `src/models/<model>.rs` file per model declared in the config,
//! rendering `model.rs.hbs` with the model's fields mapped to Rust types.

use crate::cli::config::MibloConfig;
use crate::engine::handlebars_model::send_model_handlebars;
use crate::writer::writer_models;
use handlebars::Handlebars;
use std::{error::Error, path::PathBuf};

/// Generate SQLx model struct files for every model in `miblo_config`.
///
/// Reads `model.rs.hbs` from the template directory and writes one `.rs`
/// file per model under `<project_path>/src/models/`.
///
/// # Errors
///
/// Returns an error if the template cannot be loaded or if writing any output file fails.
pub fn generate(project_path: &PathBuf, miblo_config: &MibloConfig) -> Result<(), Box<dyn Error>> {
    let mut hbs = Handlebars::new();
    let template_path = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("model.rs.hbs");

    hbs.register_template_file("model", &template_path)
        .expect("Failed to find template model hbs");

    let res = send_model_handlebars("model", None, &miblo_config.models, &hbs);

    // writer
    writer_models::writer_model(project_path, res)?;

    Ok(())
}
