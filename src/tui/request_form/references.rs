use super::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct Row {
    pub id: i64,
    pub label: String,
}
pub(super) enum Lookup {
    Loading,
    Ready(Vec<Row>),
    Unavailable,
}
pub(super) type Lookups = Arc<Mutex<BTreeMap<String, Lookup>>>;

pub(super) fn start(project: &Project, fields: &[Input], base: &str) -> Lookups {
    let targets: BTreeMap<_, _> = fields
        .iter()
        .filter_map(|input| {
            let reference = input.field.references.as_ref()?;
            let resource = project
                .resources
                .iter()
                .find(|r| r.name == reference.resource)?;
            let labels: Vec<String> = resource
                .fields
                .iter()
                .filter(|f| !f.private && matches!(f.kind, FieldType::String | FieldType::Text))
                .map(|f| f.name.clone())
                .collect();
            Some((resource.name.clone(), (resource.path.clone(), labels)))
        })
        .collect();
    let state: Lookups = Arc::new(Mutex::new(
        targets
            .keys()
            .map(|name| (name.clone(), Lookup::Loading))
            .collect(),
    ));
    if targets.is_empty() {
        return state;
    }
    let output = Arc::clone(&state);
    let base = base.to_owned();
    thread::spawn(move || {
        let start = Instant::now();
        for (name, (path, labels)) in targets {
            // Closing the form drops its last reader. Lookups never issue a write request.
            if Arc::strong_count(&output) == 1 {
                break;
            }
            let result = if start.elapsed() < Duration::from_secs(10) {
                let reply = http::send_with_timeout(
                    &base,
                    http::Request {
                        method: "GET".into(),
                        path: format!("{path}?limit=20"),
                        body: String::new(),
                    },
                    Duration::from_secs(2),
                );
                if reply.status == Some(200) && !reply.truncated {
                    serde_json::from_str::<Vec<Value>>(&reply.body)
                        .ok()
                        .map(|values| {
                            values
                                .into_iter()
                                .filter_map(|value| {
                                    let id = value.get("id")?.as_i64()?;
                                    let label = labels
                                        .iter()
                                        .find_map(|name| value.get(name)?.as_str())
                                        .unwrap_or("");
                                    Some(Row {
                                        id,
                                        label: format!(
                                            "#{id}  {}",
                                            super::super::clean(
                                                &label.chars().take(48).collect::<String>()
                                            )
                                        ),
                                    })
                                })
                                .collect()
                        })
                        .map_or(Lookup::Unavailable, Lookup::Ready)
                } else {
                    Lookup::Unavailable
                }
            } else {
                Lookup::Unavailable
            };
            output
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(name, result);
        }
    });
    state
}
