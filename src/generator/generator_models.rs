use handlebars::Handlebars;
use crate::{cli::config::MibloConfig, engine::model_template::send_model_handelbars};

 pub fn generate_model(miblo_config: &MibloConfig) -> Vec<(String, String)> {
   let mut handlebars = Handlebars::new();

   let template_path = miblo_config.config_dir.join(&miblo_config.template_dir).join("model.rs.hbs");

   handlebars
       .register_template_file("model", &template_path)
       .expect("Failed to register template file for handlebars");

     
   send_model_handelbars("model",None, &miblo_config.models, &handlebars)
   
}


