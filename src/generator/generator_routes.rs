use serde::Deserialize;
#[derive(Debug, Deserialize, Clone)]
pub struct Route {
    pub method: String,
    pub name: String,
    pub path: String,
}

pub fn generate_routes(routes: &[Route]) {
    for route in routes {
        println!("{:?}", route);
    }
}

