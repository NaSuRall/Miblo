use dotenvy::dotenv;
use sqlx::mysql::MySqlPoolOptions;
use std::env;

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::Pool<sqlx::MySql>,
}

#[tokio::main]
pub async fn main() {
    dotenv().ok();


    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not found");

    let db = MySqlPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Impossible de se connecter a Mysql");

    let state = AppState { db };

    let address = env::var("SERVER_ADDRESS").unwrap();
    let port = env::var("SERVER_PORT").unwrap();
    let server_addr = format!("{}:{}", address, port);


    let app = routes::create_router().with_state(state);
    let listener = tokio::net::TcpListener::bind(&server_addr).await.unwrap();
    println!("Server running on {}", server_addr);

    axum::serve(listener, app).await.unwrap();


}
