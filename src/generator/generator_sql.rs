use serde_json::Value;
use crate::writer::writer_sql;
use handlebars::Handlebars;

use crate::engine::{global_fn::map_type_sql, model_template::send_model_handelbars};

pub fn generator(name: &str, models: &Vec<Value>) {
 
    let mut handlebars = Handlebars::new();

    handlebars.register_template_file("sql_get", "src/templates/handlebars/rust/sql/get.sql.hbs")
        .expect("Impossible de recuperer le template file");

    handlebars.register_template_file("sql_post", "src/templates/handlebars/rust/sql/post.sql.hbs")
        .expect("Impossible de recuperer le template file");

    handlebars.register_template_file("sql_patch", "src/templates/handlebars/rust/sql/patch.sql.hbs")
        .expect("Impossible de recuprer le template file ");

    handlebars.register_template_file("sql_delete","src/templates/handlebars/rust/sql/delete.sql.hbs")
        .expect("Imposible de tecuperer le template file");
    

    let get = send_model_handelbars("sql_get", Some(map_type_sql), models, &handlebars);

    let post = send_model_handelbars("sql_post", Some(map_type_sql), models, &handlebars);

    let patch = send_model_handelbars("sql_patch", Some(map_type_sql), models, &handlebars);

    let delete = send_model_handelbars("sql_delete", Some(map_type_sql), models, &handlebars);


    
    // prendre les Donnes des fichier sql et le donner au writer
    
    let _ = writer_sql::writer(name ,get, post, patch, delete);

    // println!("get : {:?}", get);
    // println!("post: {:?}", post);
    // println!("patch: {:?}", patch);
    // println!("delete: {:?}", delete);
}
