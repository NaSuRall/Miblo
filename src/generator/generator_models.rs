use crate::cli::config::MibloConfig;
use crate::engine::handlebars_model::send_model_handlebars;
use crate::writer::writer_models;
use handlebars::Handlebars;
use std::{error::Error, path::PathBuf};

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
