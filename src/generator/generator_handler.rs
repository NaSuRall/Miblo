use handlebars::Handlebars;
use serde_json::{json,Value};

pub fn generate_handler(models: &Vec<Value>) -> Vec<(String, String)>{
    let mut results = Vec::new();
    let mut handlebars = Handlebars::new();

    handlebars
        .register_template_file("handlers", "src/templates/handlebars/rust/handlers.rs.hbs")
        .expect("Filed to register template file for handlebars...");

    for model in models {
        let name = model["name"].as_str().expect("Model name is not a string");
        let result = format!("{}{}", &name[..1].to_uppercase(), &name[1..]);
        let data = json!({
            "handler_name": result,
            "handler_name_low": name.to_lowercase()
        });

        let rendered = handlebars
            .render("handlers", &data)
            .expect("Erreur render template");


        results.push((name.to_string(), rendered));

    }
    results

}
