use super::*;
use std::collections::BTreeMap;

pub const HTTP_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CustomRoute {
    pub name: String,
    pub method: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(default)]
    pub json_body: bool,
}

impl CustomRoute {
    pub fn parameters(&self) -> Vec<&str> {
        self.path
            .split('/')
            .filter_map(|segment| segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
            .collect()
    }

    pub fn validate(&self) -> Result<()> {
        identifier(&self.name)?;
        ensure!(
            self.name == self.name.to_snake_case(),
            "Route name must be snake_case"
        );
        ensure!(
            !["error", "router"].contains(&self.name.as_str()),
            "Reserved route name: {}",
            self.name
        );
        ensure!(
            HTTP_METHODS.contains(&self.method.as_str()),
            "Unsupported HTTP method: {}",
            self.method
        );
        ensure!(
            !self.json_body || ["POST", "PUT", "PATCH", "DELETE"].contains(&self.method.as_str()),
            "JSON bodies are supported for POST, PUT, PATCH and DELETE"
        );
        ensure!(
            self.path.starts_with('/') && self.path.len() <= 256,
            "Route path must start with / (max 256 characters)"
        );
        let mut parameters = HashSet::new();
        if self.path != "/" {
            for segment in self.path[1..].split('/') {
                if let Some(parameter) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}'))
                {
                    identifier(parameter)?;
                    ensure!(
                        parameter == parameter.to_snake_case(),
                        "Path parameter must be snake_case: {parameter}"
                    );
                    ensure!(
                        parameters.insert(parameter),
                        "Duplicate path parameter: {parameter}"
                    );
                } else {
                    ensure!(
                        !segment.is_empty()
                            && segment.bytes().all(|b| b.is_ascii_lowercase()
                                || b.is_ascii_digit()
                                || matches!(b, b'_' | b'-')),
                        "Use a path such as /users/active or /users/{{id}}/activate, without query, wildcard or trailing slash"
                    );
                }
            }
        }
        Ok(())
    }
}

pub(super) fn validate(project: &Project) -> Result<()> {
    if project.routes.is_empty() {
        return Ok(());
    }
    ensure!(
        !project
            .resources
            .iter()
            .any(|r| r.module() == "custom_routes"),
        "CustomRoutes is reserved for custom route registration"
    );
    let mut paths = BTreeMap::<String, HashSet<String>>::new();
    let mut register = |method: &str, path: &str| -> Result<()> {
        let methods = paths.entry(path.to_owned()).or_default();
        ensure!(
            methods.insert(method.to_owned()),
            "Route conflict: {method} {path} already exists (GET also serves HEAD)"
        );
        if method == "GET" {
            ensure!(
                methods.insert("HEAD".into()),
                "Route conflict: GET {path} also serves HEAD"
            );
        }
        Ok(())
    };
    register("GET", "/health")?;
    for resource in &project.resources {
        for method in ["GET", "POST"] {
            register(method, &resource.path)?;
        }
        for method in ["GET", "PATCH", "DELETE"] {
            register(method, &format!("{}/{{id}}", resource.path))?;
        }
    }
    let mut names = HashSet::new();
    for route in &project.routes {
        route.validate()?;
        ensure!(
            names.insert(&route.name),
            "Duplicate route name: {}",
            route.name
        );
        if let Some(name) = &route.resource {
            let resource = project
                .resources
                .iter()
                .find(|r| &r.name == name)
                .with_context(|| format!("Unknown model: {name}"))?;
            ensure!(
                route.path == resource.path
                    || route.path.starts_with(&format!("{}/", resource.path)),
                "Route associated with {name} must start with {}",
                resource.path
            );
        }
        register(&route.method, &route.path)?;
    }
    // Use Axum's path matcher to detect incompatible parameter names/patterns.
    let mut router = matchit::Router::new();
    for path in paths.keys() {
        router
            .insert(path, ())
            .with_context(|| format!("Conflicting route path: {path}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project(method: &str, path: &str) -> Project {
        let mut project = Project::new(
            "test-api".into(),
            3000,
            vec![Resource::new(
                "User".into(),
                vec![parse_field("name:string").unwrap()],
            )],
        );
        project.routes.push(CustomRoute {
            name: "active_users".into(),
            method: method.into(),
            path: path.into(),
            ..Default::default()
        });
        project
    }
    #[test]
    fn checks_crud_health_and_parameter_conflicts() {
        for (method, path) in [
            ("GET", "/users"),
            ("HEAD", "/health"),
            ("PUT", "/users/{user_id}"),
        ] {
            assert!(project(method, path).validate().is_err(), "{method} {path}");
        }
        for (method, path) in [
            ("GET", "/users/active"),
            ("PUT", "/users/{id}"),
            ("POST", "/users/{id}/activate"),
        ] {
            project(method, path).validate().unwrap();
        }
    }
    #[test]
    fn rejects_invalid_paths_and_associations() {
        for path in [
            "//users",
            "/users/",
            "/users?active=1",
            "/users/{*path}",
            "/{id}/{id}",
            "/../x",
            "/{bad-name}",
        ] {
            assert!(project("GET", path).validate().is_err(), "{path}");
        }
        let mut p = project("GET", "/groups/active");
        p.routes[0].resource = Some("User".into());
        assert!(p.validate().is_err());
    }
}
