use std::path::Path;
use std::error::Error;
use handlebars::Handlebars;
use std::fs;
use serde_json::{json, Value};

use crate::cli::config::MibloConfig;

pub fn template( project_path: &Path, name: &str, miblo_config: &MibloConfig )-> Result<(), Box<dyn Error>> {
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
        "db_host": db.DB_HOST,
        "db_port": db.DB_PORT,
        "db_database": db.DB_DATABASE,
        "db_username": db.DB_USERNAME,
        "db_password": db.DB_PASSWORD,
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
        ("src/models/login.rs", "login_model.rs.hbs")
    ];
    for (output, template) in templates {
        render_and_write(&mut hbs, output, template, &data, project_path, miblo_config)?;
    }

    Ok(())
}


fn render_and_write( hbs: &mut Handlebars,output: &str,template_file: &str,data: &Value,base: &Path, miblo_config: &MibloConfig)
 -> Result<(), Box<dyn std::error::Error>> {

    let template_path = miblo_config.config_dir.join(&miblo_config.template_dir).join(template_file);
    hbs.register_template_file(template_file, &template_path)?;
    let rendered = hbs.render(template_file, data)?;
    let destination = base.join(output);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, rendered)?;
    Ok(())
}
