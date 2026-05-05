use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub DB_CONNECTION: String,
    pub DB_HOST: String,
    pub DB_PORT: u16,
    pub DB_DATABASE: String,
    pub DB_USERNAME: String,
    pub DB_PASSWORD: String,
}
