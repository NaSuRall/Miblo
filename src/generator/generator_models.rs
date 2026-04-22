use handlebars::Handlebars;
use serde_json::{Value};
use crate::engine::model_template::send_model_handelbars;

 pub fn generate_model(models: &Vec<Value>) -> Vec<(String, String)> {
   //let mut results = Vec::new();
   let mut handlebars = Handlebars::new();

   handlebars
       .register_template_file("model", "src/templates/handlebars/rust/model.rs.hbs")
       .expect("Failed to register template file for handlebars");

     
   send_model_handelbars("model",None, models, &handlebars)
   
}


