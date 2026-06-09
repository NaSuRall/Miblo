//! Handlebars rendering helpers for model-based templates.

use handlebars::Handlebars;
use crate::{engine::type_rust::map_type, models::models_models::Model};
use serde_json::json;

/// The result of rendering a Handlebars template for a single model.
pub struct RenderedModel {
    /// Lowercase model name used as the file-system key (e.g. `"user"`).
    pub name: String,
    /// Fully rendered file content ready to be written to disk.
    pub content: String,
}

/// Render `name_template` once for every model in `models`.
///
/// Each model's fields are mapped through [`map_type`] first, and then optionally
/// through a secondary `type_fn` (e.g. [`crate::engine::type_rust::map_type_sql`]).
///
/// # Arguments
///
/// * `name_template` – key under which the template was registered in `registry`.
/// * `type_fn` – optional second-pass type mapper applied after [`map_type`].
/// * `models` – slice of models from [`crate::cli::config::MibloConfig`].
/// * `registry` – pre-populated Handlebars registry.
///
/// # Panics
///
/// Panics if the template registered under `name_template` fails to render.
pub fn send_model_handlebars(
    name_template: &str,
    type_fn: Option<fn(&str) -> &str>,
    models: &Vec<Model>,
    registry: &Handlebars,
) -> Vec<RenderedModel> {
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
