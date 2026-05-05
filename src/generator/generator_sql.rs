use std::error::Error;
use std::path::PathBuf;
use handlebars::Handlebars;
use crate::cli::config::MibloConfig;
use crate::engine::handlebars_model::send_model_handlebars;
use crate::engine::type_rust::map_type_sql;
use crate::writer::writer_sql;

pub fn generate(project_path: &PathBuf, miblo_config: &MibloConfig ) -> Result<(), Box<dyn Error>>{

    let mut hbs = Handlebars::new();

    let templates = [
        ("sql_get", "sql/get.sql.hbs"),
        ("sql_post", "sql/post.sql.hbs"),
        ("sql_patch", "sql/patch.sql.hbs"),
        ("sql_delete", "sql/delete.sql.hbs")
    ];

    for (name , path) in templates {
        let full_path = miblo_config
            .config_dir
            .join(&miblo_config.template_dir)
            .join(path);

        hbs.register_template_file(name, full_path)
            .expect("Failed to load template sql");
    }

    let mut res = Vec::new();

    for template in ["sql_get", "sql_post", "sql_patch", "sql_delete"] {
        let result = send_model_handlebars(
            template,
            Some(map_type_sql),
            &miblo_config.models,
            &hbs,
        );

        for model in result {
            res.push((template.replace("sql_", ""), model));
        }
    }

     writer_sql::write(project_path, res).expect("Failed to write sql file");

    Ok(())
}
