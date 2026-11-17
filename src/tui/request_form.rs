//! Typed request editor. JSON is serialized here, never entered by the user.
mod references;
#[cfg(test)]
mod tests;

use super::{http, *};
use crate::config::{Field, FieldType};
use references::{Lookup, Lookups};
use serde_json::{Map, Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, PartialEq)]
enum Presence {
    Value,
    Null,
    Omitted,
}

#[derive(Clone)]
struct Input {
    field: Field,
    value: String,
    presence: Presence,
    optional: bool,
    automatic: bool,
}

impl Input {
    fn new(field: Field, patch: bool, suggested: bool, nonce: u64) -> Self {
        let value = example(&field, nonce);
        Self {
            optional: patch || field.nullable || suggested,
            presence: if patch {
                Presence::Omitted
            } else if field.nullable {
                Presence::Null
            } else {
                Presence::Value
            },
            field,
            value,
            automatic: true,
        }
    }
    fn parsed(&self) -> Result<Value> {
        let value = match self.field.kind {
            FieldType::String | FieldType::Text => {
                ensure!(
                    if self.field.kind == FieldType::String {
                        self.value.chars().count() <= 255
                    } else {
                        self.value.len() <= 65535
                    },
                    "{} : texte trop long",
                    self.field.name
                );
                Value::String(self.value.clone())
            }
            FieldType::Int => json!(self.value.trim().parse::<i64>().with_context(|| format!(
                "{} : saisis un entier ou choisis une ligne existante",
                self.field.name
            ))?),
            FieldType::Float => {
                let number =
                    self.value.trim().parse::<f64>().with_context(|| {
                        format!("{} : saisis un nombre décimal", self.field.name)
                    })?;
                ensure!(
                    number.is_finite(),
                    "{} : nombre décimal invalide",
                    self.field.name
                );
                json!(number)
            }
            FieldType::Bool => json!(self.value.parse::<bool>().context("Choisis vrai ou faux")?),
        };
        Ok(value)
    }
    fn label(&self) -> String {
        let shown = match self.presence {
            Presence::Omitted => "non envoyé".into(),
            Presence::Null => "NULL".into(),
            Presence::Value if self.field.private => "••••••".into(),
            Presence::Value if self.value.is_empty() && self.field.references.is_some() => {
                "choisir une ligne existante".into()
            }
            Presence::Value if self.field.kind == FieldType::Bool => {
                if self.value == "true" { "vrai" } else { "faux" }.into()
            }
            Presence::Value => super::clean(&self.value),
        };
        format!(
            "[{}] {} · {}{} = {}",
            if self.presence == Presence::Omitted {
                " "
            } else {
                "×"
            },
            self.field.name,
            type_name(self.field.kind),
            if self.optional { "" } else { " · requis" },
            shown
        )
    }
}

#[derive(Clone)]
enum Editor {
    Path(String),
    Field(usize, String),
    Name(String),
    Type(String, usize),
    Reference(usize, usize),
}

#[derive(Clone)]
pub(super) struct Form {
    request: http::Request,
    base: String,
    fields: Vec<Input>,
    selected: usize,
    editor: Option<Editor>,
    flexible: bool,
    patch: bool,
    note: String,
    message: String,
    lookups: Lookups,
}

pub(super) enum Action {
    Stay,
    Cancel,
    Send(http::Request),
}

impl Form {
    pub(super) fn new(project: &Project, request: http::Request, base: &str) -> Self {
        let route_path = request.path.split('?').next().unwrap_or(&request.path);
        // Match the route pattern before looking at its method, as Axum does.
        let mut router = matchit::Router::new();
        for route in http::routes(project) {
            let _ = router.insert(route.path.clone(), route.path);
        }
        let matched = router.at(route_path).ok();
        let pattern = matched.as_ref().map(|m| m.value.as_str());
        let custom = project
            .routes
            .iter()
            .find(|r| r.method == request.method && Some(r.path.as_str()) == pattern);
        let (resource, flexible) = if let Some(route) = custom {
            (
                route
                    .resource
                    .as_ref()
                    .and_then(|name| project.resources.iter().find(|r| &r.name == name)),
                route.action.is_none(),
            )
        } else {
            let resource = project.resources.iter().find(|r| {
                (request.method == "POST" && pattern == Some(r.path.as_str()))
                    || (request.method == "PATCH"
                        && pattern == Some(format!("{}/{{id}}", r.path).as_str()))
            });
            (resource, resource.is_none())
        };
        let patch = !flexible && request.method == "PATCH";
        let nonce = next_nonce();
        let fields: Vec<_> = resource.map_or_else(Vec::new, |r| {
            r.fields
                .iter()
                .cloned()
                .map(|f| Input::new(f, patch, flexible, nonce))
                .collect()
        });
        let note = if patch {
            "Coche uniquement les champs à modifier ; les autres restent inchangés.".into()
        } else if flexible && resource.is_some() {
            "Champs proposés depuis le modèle associé ; adapte-les au contrat de cette route (+ / Suppr).".into()
        } else if flexible {
            "Corps sans contrat déclaré : ajoute des champs avec +. Miblo construit le JSON.".into()
        } else {
            "Champs préremplis depuis le modèle. Tu peux envoyer ou modifier leurs valeurs.".into()
        };
        let lookups = references::start(project, &fields, base);
        let mut form = Self {
            request,
            base: base.into(),
            fields,
            selected: 0,
            editor: None,
            flexible,
            patch,
            note,
            message: String::new(),
            lookups,
        };
        form.request.path = http::sample_path(&form.request.path);
        form
    }

    pub(super) fn sync(&mut self) {
        let data = self.lookups.lock().unwrap_or_else(|e| e.into_inner());
        for input in &mut self.fields {
            if input.automatic
                && let Some(reference) = &input.field.references
                && let Some(Lookup::Ready(rows)) = data.get(&reference.resource)
                && let Some(row) = rows.first()
            {
                input.value = row.id.to_string();
                input.automatic = false;
            }
        }
    }

    fn request(&self) -> Result<http::Request> {
        let mut body = Map::new();
        for input in &self.fields {
            match input.presence {
                Presence::Omitted => {
                    ensure!(input.optional, "{} est obligatoire", input.field.name)
                }
                Presence::Null => {
                    ensure!(
                        input.field.nullable,
                        "{} ne peut pas être NULL",
                        input.field.name
                    );
                    body.insert(input.field.name.clone(), Value::Null);
                }
                Presence::Value => {
                    body.insert(input.field.name.clone(), input.parsed()?);
                }
            }
        }
        ensure!(
            !self.patch || !body.is_empty(),
            "Choisis au moins un champ à modifier"
        );
        let request = http::Request {
            method: self.request.method.clone(),
            path: self.request.path.clone(),
            body: Value::Object(body).to_string(),
        };
        http::validate(&self.base, &request)?;
        Ok(request)
    }

    fn rows(&self) -> Vec<String> {
        let mut rows = vec![
            "▶ Envoyer la requête".into(),
            format!("Chemin : {}", self.request.path),
        ];
        rows.extend(self.fields.iter().map(Input::label));
        if self.flexible {
            rows.push("+ Ajouter un champ".into());
        }
        rows
    }

    pub(super) fn key(&mut self, key: KeyEvent) -> Result<Action> {
        self.sync();
        self.message.clear();
        if self.editor.is_some() {
            return self.edit_key(key);
        }
        match key.code {
            KeyCode::Esc => return Ok(Action::Cancel),
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1) % self.rows().len()
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = (self.selected + self.rows().len() - 1) % self.rows().len()
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.rows().len() - 1,
            KeyCode::PageDown => self.selected = (self.selected + 8).min(self.rows().len() - 1),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(8),
            KeyCode::Char('+') if self.flexible => self.editor = Some(Editor::Name(String::new())),
            KeyCode::Delete
                if self.flexible && self.selected >= 2 && self.selected - 2 < self.fields.len() =>
            {
                self.fields.remove(self.selected - 2);
                self.selected = self.selected.min(self.rows().len() - 1);
            }
            KeyCode::Char(' ') if self.selected >= 2 => {
                if let Some(input) = self.fields.get_mut(self.selected - 2) {
                    ensure!(
                        input.optional,
                        "Ce champ est obligatoire ; modifie sa valeur avec Entrée"
                    );
                    input.presence = if input.presence == Presence::Omitted {
                        if input.field.nullable {
                            Presence::Null
                        } else {
                            Presence::Value
                        }
                    } else {
                        Presence::Omitted
                    };
                }
            }
            KeyCode::Char('n') if self.selected >= 2 => {
                if let Some(input) = self.fields.get_mut(self.selected - 2) {
                    ensure!(input.field.nullable, "Ce champ n’accepte pas NULL");
                    input.presence = if input.presence == Presence::Null {
                        Presence::Value
                    } else {
                        Presence::Null
                    };
                }
            }
            KeyCode::Enter if self.selected == 0 => return Ok(Action::Send(self.request()?)),
            KeyCode::Enter if self.selected == 1 => {
                self.editor = Some(Editor::Path(self.request.path.clone()))
            }
            KeyCode::Enter => {
                if let Some(input) = self.fields.get_mut(self.selected - 2) {
                    if input.field.references.is_some() {
                        self.editor = Some(Editor::Reference(self.selected - 2, 0));
                    } else if input.field.kind == FieldType::Bool {
                        input.value = if input.value == "true" {
                            "false"
                        } else {
                            "true"
                        }
                        .into();
                        input.presence = Presence::Value;
                    } else {
                        self.editor = Some(Editor::Field(self.selected - 2, input.value.clone()));
                    }
                } else if self.flexible {
                    self.editor = Some(Editor::Name(String::new()));
                }
            }
            _ => {}
        }
        Ok(Action::Stay)
    }

    fn edit_key(&mut self, key: KeyEvent) -> Result<Action> {
        let Some(editor) = self.editor.clone() else {
            return Ok(Action::Stay);
        };
        if key.code == KeyCode::Esc {
            self.editor = None;
            return Ok(Action::Stay);
        }
        match editor {
            Editor::Type(name, mut selected) => {
                match key.code {
                    KeyCode::Down => selected = (selected + 1) % 5,
                    KeyCode::Up => selected = (selected + 4) % 5,
                    KeyCode::Enter => {
                        let field = Field {
                            name,
                            kind: [
                                FieldType::String,
                                FieldType::Text,
                                FieldType::Int,
                                FieldType::Float,
                                FieldType::Bool,
                            ][selected],
                            nullable: true,
                            unique: false,
                            private: false,
                            references: None,
                        };
                        let mut input = Input::new(field, false, true, next_nonce());
                        input.presence = Presence::Value;
                        self.fields.push(input);
                        self.selected = self.fields.len() + 1;
                        self.editor = None;
                        return Ok(Action::Stay);
                    }
                    _ => {}
                }
                self.editor = Some(Editor::Type(name, selected));
            }
            Editor::Reference(index, mut selected) => {
                let choices = self.reference_rows(index);
                selected = selected.min(choices.len() - 1);
                match key.code {
                    KeyCode::Down => selected = (selected + 1) % choices.len(),
                    KeyCode::Up => selected = (selected + choices.len() - 1) % choices.len(),
                    KeyCode::Enter => {
                        if let Some(id) = choices[selected].0 {
                            let input = &mut self.fields[index];
                            input.value = id.to_string();
                            input.presence = Presence::Value;
                            input.automatic = false;
                            self.editor = None;
                        } else {
                            self.editor =
                                Some(Editor::Field(index, self.fields[index].value.clone()));
                        }
                        return Ok(Action::Stay);
                    }
                    _ => {}
                }
                self.editor = Some(Editor::Reference(index, selected));
            }
            _ => {
                let mut value = match &editor {
                    Editor::Path(v) | Editor::Name(v) | Editor::Field(_, v) => v.clone(),
                    _ => unreachable!(),
                };
                match key.code {
                    KeyCode::Enter => {
                        match editor {
                            Editor::Path(_) => {
                                http::request_url(&self.base, &value)?;
                                self.request.path = value;
                                self.editor = None;
                            }
                            Editor::Field(index, _) => {
                                let mut input = self.fields[index].clone();
                                input.value = value;
                                input.parsed()?;
                                input.presence = Presence::Value;
                                input.automatic = false;
                                self.fields[index] = input;
                                self.editor = None;
                            }
                            Editor::Name(_) => {
                                let name = value.trim();
                                ensure!(
                                    !name.is_empty()
                                        && name.len() <= 128
                                        && !name.chars().any(char::is_control),
                                    "Indique le nom du champ"
                                );
                                ensure!(
                                    !self.fields.iter().any(|i| i.field.name == name),
                                    "Ce champ existe déjà"
                                );
                                ensure!(self.fields.len() < 64, "Maximum : 64 champs");
                                self.editor = Some(Editor::Type(name.into(), 0));
                            }
                            _ => unreachable!(),
                        }
                        return Ok(Action::Stay);
                    }
                    KeyCode::Backspace => {
                        value.pop();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        value.clear()
                    }
                    KeyCode::Char(c)
                        if !key
                            .modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                            && value.len() + c.len_utf8() <= 65535 =>
                    {
                        value.push(c)
                    }
                    _ => {}
                }
                self.editor = Some(match editor {
                    Editor::Path(_) => Editor::Path(value),
                    Editor::Name(_) => Editor::Name(value),
                    Editor::Field(i, _) => Editor::Field(i, value),
                    _ => unreachable!(),
                });
            }
        }
        Ok(Action::Stay)
    }

    fn reference_rows(&self, index: usize) -> Vec<(Option<i64>, String)> {
        let reference = &self.fields[index]
            .field
            .references
            .as_ref()
            .expect("reference input")
            .resource;
        let data = self.lookups.lock().unwrap_or_else(|e| e.into_inner());
        let mut rows = match data.get(reference) {
            Some(Lookup::Ready(rows)) => {
                rows.iter().map(|r| (Some(r.id), r.label.clone())).collect()
            }
            _ => Vec::new(),
        };
        rows.push((None, "Saisir un identifiant manuellement".into()));
        rows
    }
    fn reference_status(&self, index: usize) -> String {
        let reference = &self.fields[index]
            .field
            .references
            .as_ref()
            .expect("reference input")
            .resource;
        match self
            .lookups
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(reference)
        {
            Some(Lookup::Loading) => format!("{reference} : chargement des lignes…"),
            Some(Lookup::Ready(rows)) if rows.is_empty() => {
                format!("{reference} : aucune ligne. Crée d’abord une ligne dans cette table.")
            }
            Some(Lookup::Ready(_)) => format!(
                "{reference} : premières lignes disponibles ; tu peux saisir un autre identifiant."
            ),
            _ => format!(
                "{reference} : liste indisponible. Vérifie l’API ou saisis un identifiant existant."
            ),
        }
    }
}

fn next_nonce() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    COUNTER.fetch_max(now, Ordering::Relaxed);
    COUNTER.fetch_add(1, Ordering::Relaxed) + 1
}
fn example(field: &Field, nonce: u64) -> String {
    if field.references.is_some() {
        return String::new();
    }
    match field.kind {
        FieldType::String | FieldType::Text => {
            let suffix = if field.unique || field.private {
                format!("-{nonce}")
            } else {
                String::new()
            };
            if field.name.contains("email") {
                format!("miblo{suffix}@example.test")
            } else {
                format!("exemple{suffix}")
            }
        }
        FieldType::Int => {
            if field.unique {
                nonce.to_string()
            } else {
                "1".into()
            }
        }
        FieldType::Float => {
            if field.unique {
                format!("{nonce}.5")
            } else {
                "1.5".into()
            }
        }
        FieldType::Bool => "true".into(),
    }
}
fn type_name(kind: FieldType) -> &'static str {
    match kind {
        FieldType::String => "texte",
        FieldType::Text => "texte long",
        FieldType::Int => "entier",
        FieldType::Float => "décimal",
        FieldType::Bool => "booléen",
    }
}

pub(super) fn draw(frame: &mut ratatui::Frame, form: &mut Form, colors: super::view::Palette) {
    use ratatui::{
        layout::{Constraint, Layout, Rect},
        style::{Color, Style},
        text::Line,
        widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    };
    form.sync();
    let screen = frame.area();
    let width = screen.width.saturating_sub(4).min(100);
    let content_rows = match &form.editor {
        Some(Editor::Type(_, _)) => 5,
        Some(Editor::Reference(index, _)) => form.reference_rows(*index).len(),
        Some(_) => 3,
        None => form.rows().len(),
    };
    let height = screen
        .height
        .saturating_sub(2)
        .min((content_rows as u16 + 9).clamp(14, 30));
    let area = Rect::new(
        screen.x + screen.width.saturating_sub(width) / 2,
        screen.y + screen.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let panel = colors.panel;
    let fg = colors.foreground;
    let accent = colors.accent;
    let style = Style::default().bg(panel).fg(fg);
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Préparer la requête · {} ", form.request.method))
        .style(style)
        .border_style(Style::default().fg(accent));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(
            "{} {}\n{}",
            form.request.method, form.request.path, form.base
        ))
        .style(style),
        rows[0],
    );
    let mut footer = if form.flexible {
        "↑ ↓ choisir · Entrée modifier/envoyer · Espace inclure/exclure\nn : NULL · + : champ libre · Suppr : retirer · Échap annuler"
    } else {
        "↑ ↓ choisir · Entrée modifier/envoyer · Espace inclure/exclure\nn : NULL · Début : envoyer · Échap annuler"
    }.to_owned();
    let mut note = form.note.clone();
    let (items, selected) = match &form.editor {
        Some(Editor::Type(name, index)) => {
            note = format!("Type du champ {name}");
            footer = "↑ ↓ choisir · Entrée valider · Échap retour".into();
            (
                vec!["Texte", "Texte long", "Entier", "Décimal", "Booléen"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                *index,
            )
        }
        Some(Editor::Reference(index, selected)) => {
            note = form.reference_status(*index);
            footer = "↑ ↓ choisir · Entrée sélectionner · Échap retour".into();
            (
                form.reference_rows(*index)
                    .into_iter()
                    .map(|(_, label)| label)
                    .collect(),
                *selected,
            )
        }
        Some(editor) => {
            let (title, value, private) = match editor {
                Editor::Path(value) => ("Chemin HTTP".to_owned(), value, false),
                Editor::Name(value) => ("Nom du nouveau champ".to_owned(), value, false),
                Editor::Field(index, value) => (
                    format!(
                        "{} · {}",
                        form.fields[*index].field.name,
                        type_name(form.fields[*index].field.kind)
                    ),
                    value,
                    form.fields[*index].field.private,
                ),
                _ => unreachable!(),
            };
            let value = if private {
                "•".repeat(value.chars().count())
            } else {
                super::clean(value)
            };
            let available = rows[1].width.saturating_sub(2) as usize;
            let shown = value
                .chars()
                .skip(value.chars().count().saturating_sub(available))
                .collect::<String>();
            frame.render_widget(
                Paragraph::new(vec![Line::from(title), Line::from(""), Line::from(shown)])
                    .style(style),
                rows[1],
            );
            footer = "Saisis uniquement la valeur, sans guillemets ni JSON.\nEntrée valider · Ctrl+U effacer · Échap retour".into();
            (Vec::new(), 0)
        }
        None => {
            if let Some(input) = form
                .selected
                .checked_sub(2)
                .and_then(|i| form.fields.get(i))
                && input.field.references.is_some()
            {
                note = form.reference_status(form.selected - 2);
            }
            (form.rows(), form.selected)
        }
    };
    if !items.is_empty() {
        let mut state = ListState::default().with_selected(Some(selected.min(items.len() - 1)));
        frame.render_stateful_widget(
            List::new(items.into_iter().map(ListItem::new).collect::<Vec<_>>())
                .style(style)
                .highlight_symbol("› ")
                .highlight_style(Style::default().fg(accent).bg(colors.selected)),
            rows[1],
            &mut state,
        );
    }
    let (note, color) = if form.message.is_empty() {
        (note, colors.muted)
    } else {
        (form.message.clone(), Color::LightRed)
    };
    frame.render_widget(
        Paragraph::new(note)
            .style(style.fg(color))
            .wrap(Wrap { trim: false }),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(footer)
            .style(style.fg(accent))
            .wrap(Wrap { trim: false }),
        rows[3],
    );
}

impl App {
    pub(super) fn prepare_request(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none(),
            "Attends la fin de l’action avant de préparer la requête"
        );
        let project = self.project.as_ref().context("Ouvre un projet d’abord")?;
        self.dialog = Some(forms::Dialog::Request(Box::new(Form::new(
            project,
            self.request_draft.clone(),
            &self.base_url,
        ))));
        Ok(())
    }
    pub(super) fn request_form_key(&mut self, mut form: Box<Form>, key: KeyEvent) -> Result<()> {
        match form.key(key) {
            Ok(Action::Send(request)) => {
                self.request_draft = request;
                if let Err(error) = self.launch_request() {
                    form.message = error.to_string();
                    self.dialog = Some(forms::Dialog::Request(form));
                }
            }
            Ok(Action::Cancel) => self.notice = "Requête annulée".into(),
            other => {
                if let Err(error) = other {
                    form.message = error.to_string();
                }
                self.dialog = Some(forms::Dialog::Request(form));
            }
        }
        Ok(())
    }
}
