use serde::Deserialize;
use handlebars::Handlebars;
use serde_json::json;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Route {
    pub method: String,
    pub name: String,
    pub path: String,
}

pub fn generate_routes(routes: &[Route]) -> String {
 
    let mut hbs = Handlebars::new();

    hbs.register_template_file(
        "routes",
        "src/templates/handlebars/rust/routes.rs.hbs"
    ).expect("Failed to register routes template");

    let data = json!({
        "routes": routes.iter().map(|r| json!({
            "path": r.path,
            "method": r.method.to_lowercase(),
            "name": r.name.to_lowercase(),
        })).collect::<Vec<_>>()
    });
    hbs.render("routes", &data).expect("Erreur render routes")

}

