use serde_json::Value;
use std::error::Error;

use crate::generator::generator_routes::Route;

pub fn reader_json(
    json: Value,
) -> Result<(Vec<Route>, Vec<Value>, Vec<Value>, Vec<Value>), Box<dyn Error>> {
    let routes = serde_json::from_value(json["route"].clone())?;
    let models = json["models"].as_array().cloned().unwrap_or_default();
    let server = json["server"].as_array().cloned().unwrap_or_default();
    let database = json["database"].as_array().cloned().unwrap_or_default();
    Ok((routes, models, database, server))
}

