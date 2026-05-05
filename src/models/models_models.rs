use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Model {
    pub name: String,
    pub method: Vec<String>,
    pub fields: Vec<Field>,
}
// r# type car type existe deja en rust
#[derive(Debug, Deserialize)]
pub struct Field {
    pub name: String,
    pub r#type: String,
    pub primary_key: Option<bool>,
    pub auto_increment: Option<bool>,
    pub not_null: Option<bool>,
    pub unique: Option<bool>,
}
