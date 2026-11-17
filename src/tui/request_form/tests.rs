use super::*;
use crate::config::{CustomRoute, Resource, parse_field};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::{Duration, Instant},
};

fn project() -> Project {
    Project::new(
        "request-test".into(),
        3000,
        vec![Resource::new(
            "User".into(),
            [
                "email:string:unique",
                "age:int",
                "score:float",
                "active:bool",
                "note:text?",
                "token:string:private",
            ]
            .iter()
            .map(|f| parse_field(f).unwrap())
            .collect(),
        )],
    )
}
fn form(method: &str, path: &str) -> Form {
    Form::new(
        &project(),
        http::Request {
            method: method.into(),
            path: path.into(),
            body: "{}".into(),
        },
        "http://127.0.0.1:1",
    )
}
fn key(form: &mut Form, code: KeyCode) -> Action {
    form.key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap()
}

#[test]
fn post_is_ready_to_send_with_typed_values_and_fresh_unique_examples() {
    let mut a = form("POST", "/users");
    let Action::Send(request) = key(&mut a, KeyCode::Enter) else {
        panic!("POST must be ready without editing JSON")
    };
    let body: Value = serde_json::from_str(&request.body).unwrap();
    assert!(body["email"].as_str().unwrap().ends_with("@example.test"));
    assert!(body["age"].is_i64() && body["score"].is_f64() && body["active"].is_boolean());
    assert!(body["note"].is_null() && body.get("id").is_none());
    assert!(body["token"].is_string());
    assert!(
        !a.rows()
            .join("\n")
            .contains(body["token"].as_str().unwrap())
    );
    let b: Value = serde_json::from_str(&form("POST", "/users").request().unwrap().body).unwrap();
    assert_ne!(body["email"], b["email"]);
}

#[test]
fn patch_distinguishes_omitted_null_and_values_and_checks_types() {
    let mut form = form("PATCH", "/users/7");
    assert!(form.request().is_err());
    form.selected = 3; // age
    key(&mut form, KeyCode::Enter);
    form.editor = Some(Editor::Field(1, "12.5".into()));
    assert!(
        form.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .is_err()
    );
    form.editor = Some(Editor::Field(1, "42".into()));
    key(&mut form, KeyCode::Enter);
    let body: Value = serde_json::from_str(&form.request().unwrap().body).unwrap();
    assert_eq!(body, json!({"age":42}));
    form.selected = 6;
    key(&mut form, KeyCode::Char('n'));
    let body: Value = serde_json::from_str(&form.request().unwrap().body).unwrap();
    assert_eq!(body, json!({"age":42,"note":null}));
    form.selected = 3;
    assert!(
        form.key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
            .is_err()
    );
    form.fields[2].value = "NaN".into();
    form.fields[2].presence = Presence::Value;
    assert!(form.request().is_err());
}

#[test]
fn custom_routes_and_free_requests_use_field_forms_without_assuming_a_crud_contract() {
    let mut project = project();
    project.routes.push(CustomRoute {
        name: "activate".into(),
        method: "POST".into(),
        path: "/users/{id}/activate".into(),
        resource: Some("User".into()),
        json_body: true,
        action: None,
    });
    let request = http::Request {
        method: "POST".into(),
        path: "/users/7/activate".into(),
        body: "{}".into(),
    };
    let form = Form::new(&project, request, "http://localhost:3000");
    assert!(form.flexible && !form.patch && form.note.contains("proposés"));
    let mut unknown = self::form("POST", "/unknown");
    assert!(unknown.fields.is_empty());
    key(&mut unknown, KeyCode::Char('+'));
    for c in "message".chars() {
        key(&mut unknown, KeyCode::Char(c));
    }
    key(&mut unknown, KeyCode::Enter);
    key(&mut unknown, KeyCode::Enter);
    key(&mut unknown, KeyCode::Enter);
    unknown.editor = Some(Editor::Field(0, "texte avec \"guillemets\" et \\".into()));
    key(&mut unknown, KeyCode::Enter);
    let body: Value = serde_json::from_str(&unknown.request().unwrap().body).unwrap();
    assert_eq!(body["message"], "texte avec \"guillemets\" et \\");
    assert!(matches!(key(&mut unknown, KeyCode::Esc), Action::Cancel));
}

#[test]
fn required_relations_select_real_ids_using_only_get() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut bytes = [0u8; 2048];
        let n = socket.read(&mut bytes).unwrap();
        assert!(String::from_utf8_lossy(&bytes[..n]).starts_with("GET /groups?limit=20 HTTP/1.1"));
        let body = r#"[{"id":57,"name":"Administrateurs","access_token":"hidden-from-picker"}]"#;
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let mut project = project();
    project.resources.push(Resource::new(
        "Group".into(),
        vec![parse_field("name:string").unwrap()],
    ));
    project.resources[0]
        .fields
        .push(parse_field("group_id:int:ref=Group").unwrap());
    let mut form = Form::new(
        &project,
        http::Request {
            method: "POST".into(),
            path: "/users".into(),
            body: "{}".into(),
        },
        &base,
    );
    assert!(form.request().is_err());
    let until = Instant::now() + Duration::from_secs(3);
    while Instant::now() < until {
        form.sync();
        if form.request().is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let body: Value = serde_json::from_str(&form.request().unwrap().body).unwrap();
    assert_eq!(body["group_id"], 57);
    let choices = form.reference_rows(form.fields.len() - 1);
    assert!(choices[0].1.contains("Administrateurs"));
    assert!(!choices[0].1.contains("hidden-from-picker"));
    worker.join().unwrap();
}

#[test]
fn respects_explicit_bodyless_routes_and_delete_json_bodies() {
    let mut project = project();
    for (method, json_body) in [("POST", false), ("DELETE", true)] {
        project.routes.push(CustomRoute {
            name: method.to_lowercase(),
            method: method.into(),
            path: "/users/{id}/action".into(),
            json_body,
            ..Default::default()
        });
    }
    assert!(!http::needs_body_form(
        Some(&project),
        &http::Request {
            method: "POST".into(),
            path: "/users/42/action".into(),
            body: "{}".into()
        }
    ));
    assert!(http::needs_body_form(
        Some(&project),
        &http::Request {
            method: "DELETE".into(),
            path: "/users/42/action?force=true".into(),
            body: String::new()
        }
    ));
    assert!(http::needs_body_form(
        Some(&project),
        &http::Request {
            method: "POST".into(),
            path: "/users".into(),
            body: String::new()
        }
    ));
}

#[test]
fn model_route_forms_use_the_actual_create_and_partial_update_contracts() {
    use crate::config::RouteAction;
    let mut p = project();
    for action in [RouteAction::Create, RouteAction::Update] {
        p.routes.push(CustomRoute {
            name: action.arg().into(),
            method: action.method().into(),
            path: if action.needs_id() {
                "/members/{id}"
            } else {
                "/members"
            }
            .into(),
            resource: Some("User".into()),
            json_body: true,
            action: Some(action),
        });
    }
    let create = Form::new(
        &p,
        http::Request {
            method: "POST".into(),
            path: "/members".into(),
            body: String::new(),
        },
        "http://localhost:1",
    );
    assert!(!create.flexible && !create.patch);
    let body: Value = serde_json::from_str(&create.request().unwrap().body).unwrap();
    assert!(body["email"].is_string() && body["note"].is_null());
    let mut update = Form::new(
        &p,
        http::Request {
            method: "PATCH".into(),
            path: "/members/2".into(),
            body: String::new(),
        },
        "http://localhost:1",
    );
    assert!(update.patch && !update.flexible && update.request().is_err());
    update.fields[1].presence = Presence::Value;
    update.fields[1].value = "23".into();
    assert_eq!(
        serde_json::from_str::<Value>(&update.request().unwrap().body).unwrap(),
        json!({"age":23})
    );
}
