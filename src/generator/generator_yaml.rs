use std::error::Error;
use serde_json::Value;


pub fn reader_json(json: Value)
    -> Result<(Vec<Value>, Vec<Value>, Vec<Value>, Value), Box<dyn Error>>
{
    
    let route = json["route"].as_array().cloned().unwrap_or_default();
    let models = json["models"].as_array().cloned().unwrap_or_default();
    let server = json["server"].as_array().cloned().unwrap_or_default();
    let database = json.get("SERV_PORT").cloned().unwrap_or(Value::Null);

    // donner les valeur pour les differente fonction dans les fichiers
    // let generated_model = generator_models::generate_model(&models);
    // println!("{:#?}", generated_model);
    // generator_routes::generate_routes(&route);
    // generator_server::generate_server(&server);
    Ok(( route, models, server, database ))
}