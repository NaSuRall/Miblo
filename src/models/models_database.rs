use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Database {
    pub db_connexion: String,
    pub db_host: String,
    pub db_port: u16,
    pub db_database: String,
    pub db_username: String,
    pub db_password: String,
}
