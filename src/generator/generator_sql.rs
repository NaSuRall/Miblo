use serde_json::Value;
use handlebars::Handlebars;

use crate::engine::{global_fn::map_type_sql, model_template::send_model_handelbars};

pub fn generator(name: &str, models: &Vec<Value>) {
 
    let mut handlebars_get = Handlebars::new();
    let mut handlebars_post = Handlebars::new();
    let mut handlebars_patch = Handlebars::new();
    let mut handlebars_delete = Handlebars::new();

    handlebars_get.register_template_file("sql_get", "src/templates/handlebars/rust/sql/get.sql.hbs")
        .expect("Impossible de recuperer le template file");

    handlebars_post.register_template_file("sql_post", "src/templates/handlebars/rust/sql/post.sql.hbs")
        .expect("Impossible de recuperer le template file");

    handlebars_patch.register_template_file("sql_patch", "src/templates/handlebars/rust/sql/patch.sql.hbs")
        .expect("Impossible de recuprer le template file ");

    handlebars_delete.register_template_file("sql_delete","src/templates/handlebars/rust/sql/delete.sql.hbs")      .expect("Imposible de tecuperer le template file");
    

    let get = send_model_handelbars("sql_get", Some(map_type_sql), models, handlebars_get);

    let post = send_model_handelbars("sql_post", Some(map_type_sql), models, handlebars_post);

    let patch = send_model_handelbars("sql_patch", Some(map_type_sql), models, handlebars_patch);

    let delete = send_model_handelbars("sql_delete", Some(map_type_sql), models, handlebars_delete);



    // prendre les Donnes des fichier sql et le donner au writer

    println!("get : {:?}", get);
    println!("post: {:?}", post);
    println!("patch: {:?}", patch);
    println!("delete: {:?}", delete);
}
