use fs_extra::file::read_to_string;
use handlebars::Handlebars;
use serde_json::{json,Value};
use std::env;

pub fn generate_handler(models: &Vec<Value> ,name: &str) -> Vec<(String, String)>{
    let mut results = Vec::new();
    let mut handlebars = Handlebars::new();

    // let current_dir = env::current_dir().expect("Impossible te trouver le dossier ");
    // let project_path = current_dir.join(name);
    // let sql_dir = project_path.join("src/sql"); 
    

    handlebars
        .register_template_file("handlers", "src/templates/handlebars/rust/handlers.rs.hbs")
        .expect("Filed to register template file for handlebars...");

    for model in models {
        let name = model["name"].as_str().expect("Model name is not a string");
        let result = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
 

        let sql_get    = format!("src/sql/{}/get.sql", name.to_lowercase());
        let sql_post   = format!("src/sql/{}/post.sql", name.to_lowercase());
        let sql_delete = format!("src/sql/{}/delete.sql", name.to_lowercase());
        let sql_patch  = format!("src/sql/{}/patch.sql", name.to_lowercase());


        let data = json!({
            "handler_name": result,
            "handler_name_low": name.to_lowercase(),
            "sql_path_get": sql_get,
            "sql_path_post": sql_post,
            "sql_path_delete": sql_delete,
            "sql_path_patch": sql_patch
        });

        let rendered = handlebars
            .render("handlers", &data)
            .expect("Erreur render template");


        results.push((name.to_string(), rendered));

    }
    results

}
