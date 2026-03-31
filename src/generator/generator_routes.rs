use serde::Deserialize;
use std::fmt::Write;
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Route {
    pub method: String,
    pub name: String,
    pub path: String,
}

pub fn generate_routes(routes: &[Route]) -> String {
 
    let mut code = String::new();


    code.push_str("use axum::{Router, routing::{get, post}};\n");
    code.push_str("use crate::handlers::*;\n\n");

    code.push_str("pub fn create_router() -> Router {\n");
    code.push_str("    Router::new()\n");

    for route in routes {
        let method = route.method.to_lowercase();
        let handler = route.name.to_lowercase();

        writeln!(
            code,
            "        .route(\"{}\", {}({}))",
            route.path, method, handler
        )
        .unwrap();
    }
        code.push_str("}\n");

    code
}

