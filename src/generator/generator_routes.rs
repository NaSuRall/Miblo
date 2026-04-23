use serde::Deserialize;
use handlebars::Handlebars;
use serde_json::json;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Route {
    pub method: String,
    pub model: String,
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
            "model_low": r.model.to_lowercase(),
            "model": capitalize(&r.model)
        })).collect::<Vec<_>>()
    });
    hbs.render("routes", &data).expect("Erreur render routes")

}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}
