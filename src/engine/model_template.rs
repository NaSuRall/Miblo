use handlebars::{Handlebars};
use serde_json::{Value, json};
use crate::engine::global_fn::map_type;


pub fn send_model_handelbars(name_template: &str ,type_fn: Option<fn(&str) -> &str >, models: &Vec<Value>, registry: Handlebars) -> Vec<(String, String)> {
    let mut results = Vec::new();

    for model in models {
        let name = model["name"].as_str().expect("Model name is not a string");
        let fields = model["fields"].as_array().expect("Field is not an array");

        // transformer fields en un tableau  compatible handlebars
        let mut field_list = Vec::new();

        for field in fields {

            let field_name = field["name"].as_str().expect("field type");
            let field_type_str = field["type"].as_str().expect("Field type");
            let rust_type = map_type(field_type_str);
            let processed_type = type_fn.map(| type_fn | type_fn(rust_type) );

            field_list.push(json!({
                   "name": field_name,
                    "type": processed_type.unwrap_or(rust_type),
                    "primary_key": field["primary_key"].as_bool().unwrap_or(false),
                    "auto_increment": field["auto_increment"].as_bool().unwrap_or(false),
                    "not_null": field["not_null"].as_bool().unwrap_or(false),
                    "null": field["null"].as_bool().unwrap_or(false),
                    "unique": field["unique"].as_bool().unwrap_or(false),
                    "default": field["default"].as_str().unwrap_or(""),
                }));
        }
        // données envoyées au template
        let data = json!({
                "model_name": name.to_lowercase(),
                "model_name_up": name,
                "fields": field_list
            });

        // rendu handlebars
        let rendered = registry
            .render(name_template, &data)
            .expect("Erreur render template");

        results.push((name.to_string(), rendered));
    }
    results
}
