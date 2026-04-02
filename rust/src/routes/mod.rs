use axum::{Router, routing::{get, post}};
use crate::handlers::*;

pub fn create_router() -> Router {
    Router::new()
        .route("/user/bonjour", get(getuser))
        .route("/users/create", post(createuser))
}
use axum::{Router, routing::{get, post}};
use crate::handlers::*;

pub fn create_router() -> Router {
    Router::new()
        .route("/user/bonjour", get(getuser))
        .route("/users/create", post(createuser))
}
