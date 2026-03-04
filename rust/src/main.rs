use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    // basic api axum
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/user", get(|| async { "Bonjour" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Serveur listener on port : 3000 .....");
    axum::serve(listener, app).await.unwrap();
}
