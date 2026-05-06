use handlebars::Handlebars;
use serde_json::json;
use std::{error::Error, path::PathBuf};

use crate::{cli::config::MibloConfig, writer::{self, writer_handlers}};

pub fn generate(project_path: &PathBuf, miblo_config: &MibloConfig) -> Result<(), Box<dyn Error>> {
    let mut hbs = Handlebars::new();
    let mut results = Vec::new();
    let template_path = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("handlers.rs.hbs");

    hbs.register_template_file("handlers", template_path)
        .expect("failed to load tempalte file for handlers");

    for model in &miblo_config.models {
        let name = &model.name;
        let fields = &model.fields;

        let mut field_names = Vec::new();

        for field in fields {
            let field_name = &field.name;
            field_names.push(json!({
                   "name": field_name,
            }));
        }

        let result = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
        let sql_get = format!("src/sql/{}/get.sql", name.to_lowercase());
        let sql_post = format!("src/sql/{}/post.sql", name.to_lowercase());
        let sql_delete = format!("src/sql/{}/delete.sql", name.to_lowercase());
        let sql_patch = format!("src/sql/{}/patch.sql", name.to_lowercase());

        let data = json!({
            "handler_name": result,
            "handler_name_low": name.to_lowercase(),
            "sql_path_get": sql_get,
            "sql_path_post": sql_post,
            "sql_path_delete": sql_delete,
            "sql_path_patch": sql_patch,
            "fields_name": field_names,
        });

        let rendered = hbs
            .render("handlers", &data)
            .expect("Erreur render template");

        results.push((name.to_string(), rendered));
    }

    // writer
    writer_handlers::writer(&project_path, results).expect("failed to write");

    Ok(())
}
