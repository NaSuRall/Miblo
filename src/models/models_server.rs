use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Server {
    pub port: Option<u16>,
    pub adress: Option<String>,
}
