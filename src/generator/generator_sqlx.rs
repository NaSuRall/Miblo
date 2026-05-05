use std::{error::Error, process::Output};
use std::process::Command;
use handlebars::Handlebars;
use std::path::PathBuf;
use crate::engine::handlebars_model::send_model_handlebars;

use crate::cli::config::MibloConfig;
use crate::engine::type_rust::map_type_sql;

pub fn generate(project_path: &PathBuf, miblo_config: &MibloConfig) -> Result<(), Box<dyn Error>> {

    let mut hbs = Handlebars::new();

    let template_path = miblo_config.config_dir.join(&miblo_config.template_dir).join("migration.sql.hbs");
    hbs.register_template_file("migration", &template_path);

    let output = Command::new("sqlx")
        .args(["migrate", "add", "init_database"])
        .current_dir(project_path)
        .output()
        .expect("Filed to créate migration database");

    let stdout = String::from_utf8(output.stdout).expect("Failed to read migration");
    let migration_path =  stdout.split_whitespace().last().expect("failed to get name of migrations");
    let migration_dir = project_path.join(migration_path);

    let res = send_model_handlebars("migration", Some(map_type_sql), &miblo_config.models, &hbs);

    // write migration  writer (migration_dir, res);

    Ok(())
}
