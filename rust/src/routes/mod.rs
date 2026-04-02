use crate::config::AppState;
use axum::Router;

pub fn create_router() -> Router<AppState> {
    Router::new()
        
}
use axum::{Router, routing::{get, post}};
use crate::handlers::*;

pub fn create_router() -> Router {
    Router::new()
        .route("/user/bonjour", get(getuser))
        .route("/users/create", post(createuser))
}
