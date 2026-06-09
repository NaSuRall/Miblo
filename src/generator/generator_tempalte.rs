//! Project scaffold generator.
//!
//! Creates the directory tree and renders the static project files
//! (`Cargo.toml`, `main.rs`, `.env`, auth handlers, …) from Handlebars templates.

use handlebars::Handlebars;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;

use crate::cli::config::MibloConfig;

/// Scaffold the static project structure under `project_path`.
///
/// Creates `src/{routes,sql,handlers,config,models}` directories and renders
/// every static template (listed in the function body) into the project tree.
///
/// # Errors
///
/// Returns an error if any directory or file cannot be created, or if template rendering fails.
pub fn template(
    project_path: &Path,
    name: &str,
    miblo_config: &MibloConfig,
) -> Result<(), Box<dyn Error>> {
    let mut hbs = Handlebars::new();

    let dirs = [
        "src/routes",
        "src/sql",
        "src/handlers",
        "src/config",
        "src/models",
    ];

    for dir in dirs {
        fs::create_dir_all(project_path.join(dir)).expect("failed to create dir");
    }

    let db = &miblo_config.database[0];
    let server = &miblo_config.server[0];

    let data = json!({
        "project_name": name,
        "server_port": server.port,
        "server_address": server.address,
        "auth": miblo_config.auth,
        "db_host": db.db_host,
        "db_port": db.db_port,
        "db_database": db.db_database,
        "db_username": db.db_username,
        "db_password": db.db_password,
    });

    let templates = [
        ("Cargo.toml", "Cargo.toml.hbs"),
        ("src/main.rs", "main.rs.hbs"),
        ("src/config/mod.rs", "config.rs.hbs"),
        (".env", ".env.hbs"),
        ("src/handlers/login.rs", "login.rs.hbs"),
        ("src/handlers/register.rs", "register.rs.hbs"),
        ("src/models/claim.rs", "claim.rs.hbs"),
        ("src/models/register.rs", "register_model.rs.hbs"),
        ("src/models/login.rs", "login_model.rs.hbs"),
    ];
    for (output, template) in templates {
        render_and_write(
            &mut hbs,
            output,
            template,
            &data,
            project_path,
            miblo_config,
        )?;
    }

    Ok(())
}

/// Load a template file, render it with `data`, and write the result to `base/output`.
///
/// Parent directories of the destination path are created automatically.
fn render_and_write(
    hbs: &mut Handlebars,
    output: &str,
    template_file: &str,
    data: &Value,
    base: &Path,
    miblo_config: &MibloConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let template_path = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join(template_file);
    hbs.register_template_file(template_file, &template_path)?;
    let rendered = hbs.render(template_file, data)?;
    let destination = base.join(output);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, rendered)?;
    Ok(())
}
