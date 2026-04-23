use fs_extra::file::read_to_string;
use handlebars::Handlebars;
use serde_json::{json,Value};
use std::env;
use std::fs;
pub fn generate_handler(models: &Vec<Value> ,name: &str) -> Vec<(String, String)>{
    let mut results = Vec::new();
    let mut handlebars = Handlebars::new();

    let current_dir = env::current_dir().expect("Impossible te trouver le dossier ");
    let project_path = current_dir.join(name);
    let sql_dir = project_path.join("src/sql"); 
    

    handlebars
        .register_template_file("handlers", "src/templates/handlebars/rust/handlers.rs.hbs")
        .expect("Filed to register template file for handlebars...");

    for model in models {
        let name = model["name"].as_str().expect("Model name is not a string");
        let result = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
        let base_sql_path = sql_dir.join(name);


        let sql_get =  read_to_string(format!("{:?}/get.sql", base_sql_path));
        let sql_post = read_to_string(format!("{:?}/post.sql", base_sql_path));
        let sql_delete = read_to_string(format!("{:?}/delete.sql", base_sql_path));
        let sql_patch = read_to_string(format!("{:?}/patch.sql", base_sql_path));
    

        println!("{:?}", sql_get);

        let data = json!({
            "handler_name": result,
            "handler_name_low": name.to_lowercase(),
            "sql_path": base_sql_path
        });

        let rendered = handlebars
            .render("handlers", &data)
            .expect("Erreur render template");


        results.push((name.to_string(), rendered));

    }
    results

}
