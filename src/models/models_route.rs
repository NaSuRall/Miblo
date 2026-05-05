use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Route {
    pub method: String,
    pub model: String,
    pub path: String
}
