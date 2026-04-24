use handlebars::Handlebars;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::cli::config::MibloConfig;

// const
pub fn generator(
    project_path: &PathBuf,
    name: &str,
    miblo_config: &MibloConfig
) -> Result<(), Box<dyn std::error::Error>> {
    // Grace a miblo_config je peux recuprer tout la config directement
    // dans la variable miblo_config
    let mut hbs = Handlebars::new();
    // fs::create_dir_all(project_path.join("src/models"))?;
    fs::create_dir_all(project_path.join("src/routes"))?;
    // fs::create_dir_all(project_path.join("src/handlers"))?;
    fs::create_dir_all(project_path.join("src/sql"))?;
    
    let db = &miblo_config.database[0];
    let data = json!({
        "project_name": name,
        "server_port": miblo_config.server[0]["port"],
        "server_adress": miblo_config.server[1]["adress"],
        "auth": miblo_config.auth,
        "db_host":     db["DB_HOST"],
        "db_port":     db["DB_PORT"],
        "db_database": db["DB_DATABASE"],
        "db_username": db["DB_USERNAME"],
        "db_password": db["DB_PASSWORD"],
    });

    render_and_write(&mut hbs, "Cargo.toml", "Cargo.toml.hbs",&data, project_path, miblo_config)?;
    render_and_write(&mut hbs, "src/main.rs", "main.rs.hbs", &data, project_path, miblo_config)?;
    render_and_write(&mut hbs, "src/config/mod.rs", "config.rs.hbs", &data, project_path, miblo_config)?;
    render_and_write(&mut hbs, ".env", ".env.hbs", &data, project_path  ,miblo_config)?;
    render_and_write(&mut hbs, "src/handlers/login.rs", "login.rs.hbs", &data, project_path ,miblo_config )?;
    render_and_write(&mut hbs, "src/handlers/register.rs", "register.rs.hbs", &data, project_path  ,miblo_config)?;
    render_and_write(&mut hbs, "src/models/claim.rs", "claim.rs.hbs", &data, project_path  ,miblo_config)?;
    render_and_write(&mut hbs, "src/models/register.rs", "register_model.rs.hbs", &data, project_path ,miblo_config)?;
    render_and_write(&mut hbs, "src/models/login.rs", "login_model.rs.hbs", &data, project_path ,miblo_config)?;

    Ok(())
}

fn render_and_write( hbs: &mut Handlebars,output: &str,template_file: &str,data: &Value,base: &Path, miblo_config: &MibloConfig)
 -> Result<(), Box<dyn std::error::Error>> {
    //let TEMPLATE_DIR: &str = "src/templates/handlebars/rust";
    // println!("{:?}, {:?}, {}", miblo_config.config_dir, miblo_config.template_dir, template_file);
    // let template_path = format!("{:?}/{}", &miblo_config.template_dir, template_file);
    //
    // Grace au miblo_config je peux mettre le config_dir puis le template_dir 
    // ce que si on les met ensemble donne le lien direct au fichier du template 
    let template_path = miblo_config.config_dir.join(&miblo_config.template_dir).join(template_file);
    // println!("TEMPLATE : {:?}", template_path);
    hbs.register_template_file(template_file, &template_path)?;
    let rendered = hbs.render(template_file, data)?;
    let destination = base.join(output);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, rendered)?;
    Ok(())
}
