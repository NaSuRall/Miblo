use handlebars::Handlebars;
use serde_json::{json, Value};

 pub fn generate_model(models: &Vec<Value>) -> Vec<(String, String)> {
   let mut results = Vec::new();
   let mut handlebars = Handlebars::new();

   println!("current dir: {:?}", std::env::current_dir());
   handlebars
       .register_template_file("model", "src/templates/handlebars/rust/model.rs.hbs")
       .expect("Failed to register template file for handlebars");


   for model in models {
      let name = model["name"].as_str().expect("Model name is not a string");
      let fields = model["fields"].as_array().expect("Field is not an array");

      // transformer fields en un tableau  compatible handlebars
      let mut field_list = Vec::new();

      for field in fields {

         let field_name = field["name"].as_str().expect("field type");
         let field_type_str = field["type"].as_str().expect("Field type");
         let rust_type = map_type(field_type_str);

         field_list.push(json!({
                "name": field_name,
                "type": rust_type
            }));
      }
      // données envoyées au template
      let data = json!({
            "model_name": name,
            "fields": field_list
        });

      // rendu handlebars
      let rendered = handlebars
          .render("model", &data)
          .expect("Erreur render template");

      results.push((name.to_string(), rendered));
   }
   results
}

pub fn map_type(t: &str) -> &str {
   match t {
      "String" | "string" => "String",
      "Int" | "int" => "i32",
      _ => "String",
   }
}

pub fn map_type_sql(rust_type: &str) -> &str {
    match rust_type {
        "String" => "TEXT",
        "i32" => "INTEGER",
        "i64" => "BIGINT",
        "bool" => "BOOLEAN",
        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",
        _ => "TEXT", // fallback
    }
}
