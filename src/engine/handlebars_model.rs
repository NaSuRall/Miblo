use handlebars::Handlebars;
use crate::{engine::type_rust::map_type, models::models_models::Model};
use serde_json::json;
pub struct RenderedModel {
    pub name : String,
    pub content: String
}

 pub fn send_model_handlebars(
     name_template: &str,
     type_fn: Option<fn(&str) -> &str>,
     models: &Vec<Model>,
     registry: &Handlebars
 ) -> Vec<RenderedModel>{
     let mut res = Vec::new();

     for model in models {
         let mut field_list = Vec::new();

         for field in &model.fields {
             let rust_type = map_type(&field.r#type);
             let final_type = type_fn.map(|f| f(rust_type)).unwrap_or(rust_type);

             field_list.push(json!({
                 "name": field.name,
                 "type": final_type,
                 "primary_key": field.primary_key,
                 "auto_increment": field.auto_increment,
                 "not_null": field.not_null,
                 "unique": field.unique,
             }));
         }

         let data = json!({
             "model_name": model.name.to_lowercase(),
             "model_name_up": model.name,
             "fields": field_list
         });
         let rendered = registry.render(name_template, &data).expect("Template error");

         res.push(RenderedModel{
             name: model.name.to_lowercase(),
             content: rendered,
         });
     }
     res
 }
