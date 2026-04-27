use crate::{cli::config::MibloConfig, writer::writer_sql};
use handlebars::Handlebars;
use std::path::PathBuf;

use crate::engine::{global_fn::map_type_sql, model_template::send_model_handelbars};

pub fn generator(project_path: &PathBuf, miblo_config: &MibloConfig) {
    let mut handlebars = Handlebars::new();

    let template_path_get = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("sql/get.sql.hbs");
    handlebars
        .register_template_file("sql_get", template_path_get)
        .expect("Impossible de recuperer le template file");

    let template_path_post = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("sql/post.sql.hbs");
    handlebars
        .register_template_file("sql_post", template_path_post)
        .expect("Impossible de recuperer le template file");

    let template_path_patch = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("sql/patch.sql.hbs");
    handlebars
        .register_template_file("sql_patch", template_path_patch)
        .expect("Impossible de recuprer le template file ");

    let template_path_delete = miblo_config
        .config_dir
        .join(&miblo_config.template_dir)
        .join("sql/delete.sql.hbs");
    handlebars
        .register_template_file("sql_delete", template_path_delete)
        .expect("Imposible de tecuperer le template file");

    let get = send_model_handelbars(
        "sql_get",
        Some(map_type_sql),
        &miblo_config.models,
        &handlebars,
    );
    let post = send_model_handelbars(
        "sql_post",
        Some(map_type_sql),
        &miblo_config.models,
        &handlebars,
    );
    let patch = send_model_handelbars(
        "sql_patch",
        Some(map_type_sql),
        &miblo_config.models,
        &handlebars,
    );
    let delete = send_model_handelbars(
        "sql_delete",
        Some(map_type_sql),
        &miblo_config.models,
        &handlebars,
    );

    // prendre les Donnes des fichier sql et le donner au writer
    let _ = writer_sql::writer(project_path, get, post, patch, delete);
}
