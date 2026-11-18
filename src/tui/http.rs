use crate::config::{CustomRoute, FieldType, Project};
use anyhow::{Context, Result, ensure};
use reqwest::{Method, Url, blocking::Client, redirect::Policy};
use std::{
    io::Read,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const MAX_RESPONSE: u64 = 64 * 1024;

#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
}
impl Default for Request {
    fn default() -> Self {
        Self {
            method: "GET".into(),
            path: "/health".into(),
            body: String::new(),
        }
    }
}
impl Request {
    pub fn writes(&self) -> bool {
        !matches!(self.method.as_str(), "GET" | "HEAD" | "OPTIONS")
    }
}

pub fn routes(project: &Project) -> Vec<Request> {
    let mut routes = vec![Request::default()];
    for resource in &project.resources {
        let body: serde_json::Map<_, _> = resource
            .fields
            .iter()
            .map(|field| {
                let value = if field.nullable {
                    serde_json::Value::Null
                } else {
                    match field.kind {
                        FieldType::String | FieldType::Text => serde_json::json!("example"),
                        FieldType::Int => serde_json::json!(1),
                        FieldType::Float => serde_json::json!(1.0),
                        FieldType::Bool => serde_json::json!(true),
                    }
                };
                (field.name.clone(), value)
            })
            .collect();
        let body = serde_json::Value::Object(body).to_string();
        for (method, path, body) in [
            ("GET", resource.path.clone(), String::new()),
            ("POST", resource.path.clone(), body.clone()),
            ("GET", format!("{}/{{id}}", resource.path), String::new()),
            ("PATCH", format!("{}/{{id}}", resource.path), body),
            ("DELETE", format!("{}/{{id}}", resource.path), String::new()),
        ] {
            routes.push(Request {
                method: method.into(),
                path,
                body,
            });
        }
    }
    routes.extend(project.routes.iter().map(|route| Request {
        method: route.method.clone(),
        path: route.path.clone(),
        body: if route.json_body {
            "{}".into()
        } else {
            String::new()
        },
    }));
    routes
}

pub fn sample_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                "1"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Explicit custom-route body choices take priority over HTTP-method defaults.
pub(super) fn needs_body_form(project: Option<&Project>, request: &Request) -> bool {
    if let Some(route) = project.and_then(|p| custom_route(p, &request.method, &request.path)) {
        return route.json_body;
    }
    !request.body.is_empty() || matches!(request.method.as_str(), "POST" | "PATCH" | "PUT")
}

/// Match the concrete HTTP path with the same static-path priority as Axum.
/// Include CRUD paths so an unrelated method cannot fall through to a dynamic route.
pub(super) fn custom_route<'a>(
    project: &'a Project,
    method: &str,
    path: &str,
) -> Option<&'a CustomRoute> {
    let mut router = matchit::Router::new();
    for route in routes(project) {
        let _ = router.insert(route.path.clone(), route.path);
    }
    let path = path.split('?').next().unwrap_or(path);
    let matched = router.at(path).ok()?;
    project.routes.iter().find(|route| {
        &route.path == matched.value
            && (route.method == method || (method == "HEAD" && route.method == "GET"))
    })
}

pub fn request_url(base: &str, path: &str) -> Result<Url> {
    ensure!(
        path.starts_with('/')
            && !path.starts_with("//")
            && !path.contains(['\\', '{', '}'])
            && !path.chars().any(char::is_control),
        trf!(
            "Enter a local path such as /users/1 (replace the parameters)",
            "Indique un chemin local comme /users/1 (remplace les paramètres)"
        )
    );
    let base = Url::parse(base).context(tr!("Invalid API address", "Adresse de l’API invalide"))?;
    ensure!(
        matches!(base.scheme(), "http" | "https")
            && base.username().is_empty()
            && base.password().is_none(),
        trf!("Invalid API HTTP address", "Adresse HTTP de l’API invalide")
    );
    let url = base.join(path)?;
    ensure!(
        url.origin() == base.origin() && url.fragment().is_none(),
        trf!(
            "The request must target this API, without a fragment",
            "La requête doit cibler cette API, sans fragment"
        )
    );
    Ok(url)
}

pub fn validate(base: &str, request: &Request) -> Result<()> {
    request_url(base, &request.path)?;
    ensure!(
        ["GET", "POST", "PATCH", "PUT", "DELETE", "HEAD", "OPTIONS"]
            .contains(&request.method.as_str()),
        trf!(
            "Unsupported HTTP method",
            "Méthode HTTP non prise en charge"
        )
    );
    ensure!(
        request.body.len() <= 64 * 1024,
        trf!("JSON body is too large", "Corps JSON trop volumineux")
    );
    if !request.body.trim().is_empty() {
        serde_json::from_str::<serde_json::Value>(&request.body)
            .context(tr!("Invalid JSON body", "Corps JSON invalide"))?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub method: String,
    pub path: String,
    pub status: Option<u16>,
    pub elapsed_ms: u128,
    pub content_type: String,
    pub body: String,
    pub truncated: bool,
}

pub fn send(base: &str, request: Request) -> Reply {
    send_with_timeout(base, request, Duration::from_secs(10))
}

pub(super) fn send_with_timeout(base: &str, request: Request, timeout: Duration) -> Reply {
    let start = Instant::now();
    let mut reply = Reply {
        method: request.method.clone(),
        path: request.path.clone(),
        status: None,
        elapsed_ms: 0,
        content_type: String::new(),
        body: String::new(),
        truncated: false,
    };
    let result = (|| -> Result<()> {
        validate(base, &request)?;
        let client = Client::builder()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(3))
            .redirect(Policy::none())
            .no_proxy()
            .build()?;
        let mut builder = client.request(
            Method::from_bytes(request.method.as_bytes())?,
            request_url(base, &request.path)?,
        );
        if !request.body.is_empty() {
            builder = builder
                .header("Content-Type", "application/json")
                .body(request.body);
        }
        let response = builder.send().map_err(|error| {
            anyhow::anyhow!(if error.is_timeout() {
                tr!("HTTP timeout (10 s)", "Délai HTTP dépassé (10 s)")
            } else {
                tr!(
                    "HTTP connection failed; check the address and start the API",
                    "Connexion HTTP impossible ; vérifie l’adresse et le lancement de l’API"
                )
            })
        })?;
        reply.status = Some(response.status().as_u16());
        reply.content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .into();
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE + 1)
            .read_to_end(&mut bytes)
            .context(tr!(
                "Response reading interrupted",
                "Lecture de la réponse interrompue"
            ))?;
        reply.truncated = bytes.len() as u64 > MAX_RESPONSE;
        bytes.truncate(MAX_RESPONSE as usize);
        reply.body = if !reply.truncated {
            serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()
                .and_then(|value| serde_json::to_string_pretty(&value).ok())
                .unwrap_or_else(|| String::from_utf8_lossy(&bytes).into_owned())
        } else {
            String::from_utf8_lossy(&bytes).into_owned()
        };
        Ok(())
    })();
    if let Err(error) = result {
        reply.body = format!("{error:#}");
        reply.status = None;
    }
    reply.elapsed_ms = start.elapsed().as_millis();
    reply
}

pub struct Tester {
    pub pending: bool,
    pub reply: Option<Reply>,
    pub lines: Vec<String>,
    pub scroll: usize,
    sender: mpsc::SyncSender<Reply>,
    receiver: mpsc::Receiver<Reply>,
}
impl Default for Tester {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        Self {
            pending: false,
            reply: None,
            lines: vec![],
            scroll: 0,
            sender,
            receiver,
        }
    }
}
impl Tester {
    pub fn start(&mut self, base: &str, request: Request) -> Result<()> {
        ensure!(
            !self.pending,
            trf!(
                "An HTTP request is already running",
                "Une requête HTTP est déjà en cours"
            )
        );
        validate(base, &request)?;
        self.pending = true;
        self.reply = None;
        self.lines.clear();
        self.scroll = 0;
        let base = base.to_owned();
        let sender = self.sender.clone();
        thread::spawn(move || {
            let _ = sender.send(send(&base, request));
        });
        Ok(())
    }
    pub fn tick(&mut self) -> bool {
        if let Ok(reply) = self.receiver.try_recv() {
            self.pending = false;
            self.lines = reply.body.lines().map(super::clean).collect();
            if reply.truncated {
                self.lines
                    .push(tr!("… response limited to 64 KiB", "… réponse limitée à 64 Kio").into());
            }
            if self.lines.is_empty() {
                self.lines
                    .push(tr!("(empty response body)", "(réponse sans corps)").into());
            }
            self.reply = Some(reply);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};
    #[test]
    fn confines_requests_and_validates_json() {
        let base = "http://127.0.0.1:3000";
        for path in [
            "//outside.test/",
            "/\\outside.test/",
            "https://outside.test/",
            "/users/{id}",
            "/users#secret",
        ] {
            assert!(request_url(base, path).is_err(), "{path}");
        }
        assert_eq!(
            request_url(base, "/users?limit=1").unwrap().path(),
            "/users"
        );
        assert!(
            validate(
                base,
                &Request {
                    method: "POST".into(),
                    path: "/users".into(),
                    body: "{broken}".into()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn displays_http_errors_and_does_not_follow_redirects() {
        for (status, body) in [
            (404, "{\"error\":\"missing\"}"),
            (302, "redirect"),
            (204, ""),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let worker = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = [0; 2048];
                assert!(stream.read(&mut bytes).unwrap() > 0);
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let reply = send(&base, Request::default());
            assert_eq!(reply.status, Some(status));
            if status == 404 {
                assert!(reply.body.contains("missing"));
            }
            worker.join().unwrap();
        }
    }
}
