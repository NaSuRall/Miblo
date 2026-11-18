use super::*;
use crate::config::{Field, FieldType, OnDelete, Reference, Resource, parse_field, project_name};

#[derive(Clone, Copy)]
pub(super) enum InputKind {
    RoutePath,
    RouteName,
    Command,
    Open,
    NewName,
    NewOutput,
    NewPort,
    NewLocal,
    NewUrl,
    ResourceName,
    FieldName,
    Collection,
    DbHost,
    DbPort,
    DbName,
    DbUser,
    DbPassword,
    HttpPath,
    RelationName,
    TemplateLocal,
    TemplateUrl,
}
#[derive(Clone, Copy)]
pub(super) enum ChoiceKind {
    RouteRecovery,
    RouteResource,
    RouteAction,
    RouteMethod,
    RouteBody,
    RouteConfirm,
    Palette,
    Language,
    Source,
    FieldType,
    NextField,
    NewConfirm,
    NewRetry,
    AddConfirm,
    EditModel,
    EditAction,
    EditConfirm,
    Database,
    DbMenu,
    DbConfirm(Action),
    DbSave,
    HttpMethod,
    HttpConfirm,
    RelationTarget,
    RelationDelete,
    TemplateSource,
    TemplateConfirm,
}
#[derive(Clone)]
pub(super) enum Dialog {
    Request(Box<super::request_form::Form>),
    Input {
        kind: InputKind,
        title: String,
        value: String,
    },
    Select {
        kind: ChoiceKind,
        title: String,
        description: Vec<String>,
        items: Vec<String>,
        selected: usize,
        scroll: u16,
    },
    Options {
        items: Vec<(String, bool)>,
        selected: usize,
        unique_available: bool,
        private_available: bool,
    },
    Help,
}
#[derive(Default)]
pub(super) struct NewDraft {
    name: String,
    output: PathBuf,
    port: u16,
    source: usize,
    location: String,
    local_hint: Option<PathBuf>,
    database: Database,
}
#[derive(Default)]
pub(super) struct AddDraft {
    name: String,
    path: String,
    fields: Vec<Field>,
    pending: Option<Field>,
    editing: bool,
}
#[derive(Default)]
pub(super) struct TemplateDraft {
    source: usize,
    location: String,
}

impl App {
    pub(super) fn start_route_test(&mut self) -> Result<()> {
        ensure!(
            !self.tester.pending,
            trf!(
                "An HTTP request is already running",
                "Une requête HTTP est déjà en cours"
            )
        );
        self.request_draft = self
            .routes
            .get(self.route_selected)
            .context(tr!(
                "Open a project to test its routes",
                "Ouvre un projet pour tester ses routes"
            ))?
            .clone();
        self.show_schema = false;
        if http::needs_body_form(self.project.as_ref(), &self.request_draft) {
            return self.prepare_request();
        }
        self.request_draft.body.clear();
        // Every request exposes its path, including query parameters, before sending.
        self.input(
            InputKind::HttpPath,
            tr!(
                "HTTP path — replace parameters with your values",
                "Chemin HTTP — remplace les paramètres par tes valeurs"
            ),
            &http::sample_path(&self.request_draft.path),
        );
        Ok(())
    }
    pub(super) fn custom_request(&mut self) -> Result<()> {
        ensure!(
            self.project.is_some(),
            trf!("Open a project first", "Ouvre un projet d’abord")
        );
        self.show_schema = false;
        ensure!(
            !self.tester.pending,
            trf!(
                "An HTTP request is already running",
                "Une requête HTTP est déjà en cours"
            )
        );
        self.request_draft = http::Request::default();
        self.choose(
            ChoiceKind::HttpMethod,
            tr!("HTTP method", "Méthode HTTP"),
            &["GET", "POST", "PATCH", "PUT", "DELETE", "HEAD", "OPTIONS"],
            vec![format!("API : {}", self.base_url)],
            0,
        );
        Ok(())
    }
    fn confirm_request(&mut self) -> Result<()> {
        http::validate(&self.base_url, &self.request_draft)?;
        if self.request_draft.writes() {
            self.choose(
                ChoiceKind::HttpConfirm,
                tr!("Send this request?", "Envoyer cette requête ?"),
                &[tr!("Send", "Envoyer"), tr!("Cancel", "Annuler")],
                vec![
                    format!("{} {}", self.request_draft.method, self.request_draft.path),
                    format!("API : {}", self.base_url),
                    tr!(
                        "This request may change data in this API.",
                        "Cette requête peut modifier les données de cette API."
                    )
                    .into(),
                ],
                1,
            );
            Ok(())
        } else {
            self.launch_request()
        }
    }
    pub(super) fn launch_request(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none(),
            trf!(
                "Wait for the current action before testing the route",
                "Attends la fin de l’action avant de tester la route"
            )
        );
        self.tester
            .start(&self.base_url, self.request_draft.clone())?;
        self.dialog = None;
        self.focus = Focus::Response;
        self.notice = trf!(
            "{} {} · request in progress…",
            "{} {} · requête en cours…",
            self.request_draft.method,
            self.request_draft.path
        );
        Ok(())
    }
    pub(super) fn input(&mut self, kind: InputKind, title: &str, value: &str) {
        self.dialog = Some(Dialog::Input {
            kind,
            title: title.into(),
            value: value.into(),
        });
    }
    pub(super) fn choose(
        &mut self,
        kind: ChoiceKind,
        title: &str,
        items: &[&str],
        description: Vec<String>,
        selected: usize,
    ) {
        self.dialog = Some(Dialog::Select {
            kind,
            title: title.into(),
            description,
            items: items.iter().map(|v| (*v).into()).collect(),
            selected,
            scroll: 0,
        });
    }
    pub(super) fn palette(&mut self) {
        self.choose(
            ChoiceKind::Palette,
            tr!("Actions", "Actions"),
            &[
                tr!("Create an API", "Créer une API"),
                tr!("Open a project", "Ouvrir un projet"),
                tr!(
                    "Add a model / table + CRUD",
                    "Ajouter un modèle / une table + CRUD"
                ),
                tr!("Start the API", "Lancer l’API"),
                tr!("Stop the API", "Arrêter l’API"),
                tr!("Check compilation", "Vérifier la compilation"),
                tr!("Run tests", "Exécuter les tests"),
                tr!("Project diagnostics", "Diagnostic du projet"),
                tr!(
                    "Open file in my editor",
                    "Ouvrir le fichier dans mon éditeur"
                ),
                tr!("Refresh files", "Actualiser les fichiers"),
                tr!("Change theme", "Changer de thème"),
                tr!("Help", "Aide"),
                tr!("Quit", "Quitter"),
                tr!(
                    "Database: connection / migrations",
                    "Base de données : connexion / migrations"
                ),
                tr!("Test selected route", "Tester la route sélectionnée"),
                tr!("Custom HTTP request", "Requête HTTP personnalisée"),
                tr!(
                    "Show / hide code preview",
                    "Afficher / masquer l’aperçu du code"
                ),
                tr!(
                    "View database schema",
                    "Voir le schéma de la base de données"
                ),
                tr!(
                    "Update templates for new models and routes",
                    "Mettre à jour les templates des futurs modèles et routes"
                ),
                tr!(
                    "Edit an existing model: fields / relationships",
                    "Modifier un modèle existant : champs / relations"
                ),
                tr!(
                    "Create a model / custom route",
                    "Créer une route de modèle / personnalisée"
                ),
                "Language / Langue",
            ],
            Vec::new(),
            0,
        );
    }
    pub(super) fn start_new(&mut self) -> Result<()> {
        ensure!(
            self.job.is_none() && self.api.is_none() && !self.tester.pending,
            trf!(
                "Stop the API and wait for the current action before creating another project",
                "Arrête l’API et attends la fin de l’action avant de créer un autre projet"
            )
        );
        self.new_draft = NewDraft::default();
        self.input(
            InputKind::NewName,
            tr!("New API name", "Nom de la nouvelle API"),
            "my-api",
        );
        Ok(())
    }

    fn choose_source(&mut self) {
        let local = self.new_draft.local_hint.is_some();
        self.choose(
            ChoiceKind::Source,
            tr!("Template source", "Source du template"),
            &[
                tr!(
                    "GitHub Releases — published archive required",
                    "GitHub Releases — archive publiée requise"
                ),
                if local {
                    tr!(
                        "Local directory — suggested path",
                        "Dossier local — chemin proposé"
                    )
                } else {
                    tr!("Local directory", "Dossier local")
                },
                tr!(
                    "Custom server — archive URL",
                    "Serveur personnalisé — URL d’archive"
                ),
            ],
            vec![
                trf!(
                    "Rust / Axum / {} • templates downloaded on demand",
                    "Rust / Axum / {} • templates téléchargés à la demande",
                    self.new_draft.database.name()
                ),
                tr!(
                    "Remote sources require a .tar.gz archive and its .sha256 file.",
                    "La source distante exige une archive .tar.gz et son .sha256."
                )
                .into(),
            ],
            0,
        );
    }

    pub(super) fn creation_failed(&mut self) {
        self.notice = tr!(
            "Creation failed • change the source without starting over",
            "Création échouée • Tu peux changer de source sans tout ressaisir"
        )
        .into();
        let error = self
            .logs
            .iter()
            .rev()
            .find_map(|line| line.strip_prefix("[create] Error: "))
            .unwrap_or(tr!(
                "Check the logs for error details.",
                "Consulte les logs pour le détail de l’erreur."
            ))
            .to_owned();
        self.choose(
            ChoiceKind::NewRetry,
            tr!("Creation failed", "La création a échoué"),
            &[
                tr!("Change template source", "Changer de source du template"),
                tr!("Retry", "Réessayer"),
                tr!("Cancel", "Annuler"),
            ],
            vec![
                tr!(
                    "The name, directory and port have been kept.",
                    "Le nom, le dossier et le port ont été conservés."
                )
                .into(),
                error,
                tr!(
                    "If the release is unavailable, choose a local directory.",
                    "Si la release est introuvable, choisis un dossier local."
                )
                .into(),
            ],
            0,
        );
    }
    pub(super) fn start_add(&mut self) -> Result<()> {
        let root = self.ready_for_job()?;
        generation::inspect(root)?;
        self.add_draft = AddDraft::default();
        self.input(
            InputKind::ResourceName,
            tr!(
                "Model name (e.g. User or Product)",
                "Nom du modèle (ex. User ou Product)"
            ),
            "",
        );
        Ok(())
    }

    pub(super) fn start_model_edit(&mut self) -> Result<()> {
        ensure!(
            self.api.is_none(),
            trf!(
                "Stop the API with s before editing a model",
                "Arrête l’API avec s avant de modifier un modèle"
            )
        );
        let (project, _) = generation::inspect(self.ready_for_job()?)?;
        ensure!(
            !project.resources.is_empty(),
            trf!(
                "Add a model first with a",
                "Ajoute d’abord un modèle avec a"
            )
        );
        let names: Vec<_> = project.resources.iter().map(|r| r.name.as_str()).collect();
        self.choose(
            ChoiceKind::EditModel,
            tr!("Edit an existing model", "Modifier un modèle existant"),
            &names,
            vec![
                tr!(
                    "Add optional fields / relationships without deleting data.",
                    "Ajout de champs / relations facultatifs, sans supprimer les données."
                )
                .into(),
            ],
            self.resource_selected.min(names.len() - 1),
        );
        self.project = Some(project);
        Ok(())
    }

    fn existing_fields(&self) -> Vec<Field> {
        if !self.add_draft.editing {
            return vec![];
        }
        self.project
            .as_ref()
            .and_then(|p| p.resources.iter().find(|r| r.name == self.add_draft.name))
            .map_or(vec![], |r| r.fields.clone())
    }

    fn validate_draft_fields(&self, added: &[Field]) -> Result<()> {
        let mut fields = self.existing_fields();
        fields.extend_from_slice(added);
        Resource::new(self.add_draft.name.clone(), fields).validate()
    }

    fn confirm_model_edit(&mut self) -> Result<()> {
        let project = self
            .project
            .as_ref()
            .context(tr!("No project", "Aucun projet"))?;
        let resource = self.draft_resource(&self.add_draft.path);
        self.validate_draft_fields(&resource.fields)?;
        let sql = generation::alteration_sql(&resource, &resource.fields, project.database())?;
        let mut description = vec![
            trf!(
                "{} · {} new field(s)",
                "{} · {} nouveau(x) champ(s)",
                resource.name,
                resource.fields.len()
            ),
            tr!(
                "Existing rows are preserved; new fields will contain NULL.",
                "Lignes conservées ; les nouveaux champs contiendront NULL."
            )
            .into(),
            tr!(
                "Custom code is merged and replaced files are backed up.",
                "Fusion du code personnalisé et sauvegarde des fichiers remplacés."
            )
            .into(),
            tr!(
                "A migration will be created. Apply it with b → migrations.",
                "Une migration sera créée. Applique-la ensuite avec b → migrations."
            )
            .into(),
        ];
        description.extend(
            sql.lines()
                .filter(|l| !l.starts_with("--"))
                .map(str::to_owned),
        );
        self.choose(
            ChoiceKind::EditConfirm,
            tr!(
                "Generate the model update?",
                "Générer la modification du modèle ?"
            ),
            &[
                tr!(
                    "Generate migration and code",
                    "Générer la migration et le code"
                ),
                tr!("Cancel", "Annuler"),
            ],
            description,
            0,
        );
        Ok(())
    }

    pub(super) fn dialog_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(mut dialog) = self.dialog.take() else {
            return Ok(());
        };
        if let Dialog::Request(form) = dialog {
            return self.request_form_key(form, key);
        }
        if key.code == KeyCode::Esc {
            self.resume_route_after_template_update = false;
            self.db_draft.password.clear();
            self.notice = tr!(
                "Action cancelled • No files changed",
                "Action annulée • Aucun fichier modifié"
            )
            .into();
            return Ok(());
        }
        let previous = dialog.clone();
        let result = match &mut dialog {
            Dialog::Request(_) => unreachable!(),
            Dialog::Help => {
                return Ok(());
            }
            Dialog::Input { kind, value, .. } => match key.code {
                KeyCode::Enter => self.submit_input(
                    *kind,
                    if matches!(kind, InputKind::DbPassword) {
                        value
                    } else {
                        value.trim()
                    },
                ),
                KeyCode::Backspace => {
                    value.pop();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    value.clear();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    let limit = 4096;
                    if value.len() + c.len_utf8() <= limit {
                        value.push(c);
                    }
                    self.dialog = Some(dialog);
                    Ok(())
                }
                _ => {
                    self.dialog = Some(dialog);
                    Ok(())
                }
            },
            Dialog::Select {
                kind,
                selected,
                items,
                scroll,
                ..
            } => match key.code {
                KeyCode::PageDown => {
                    *scroll = scroll.saturating_add(5);
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::PageUp => {
                    *scroll = scroll.saturating_sub(5);
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Enter => self.submit_choice(*kind, *selected),
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1) % items.len();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = (*selected + items.len() - 1) % items.len();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                _ => {
                    self.dialog = Some(dialog);
                    Ok(())
                }
            },
            Dialog::Options {
                items,
                selected,
                unique_available,
                private_available,
            } => match key.code {
                KeyCode::Enter => self.submit_options(items, *unique_available, *private_available),
                KeyCode::Char(' ') => {
                    if !(self.add_draft.editing && *selected == 0) {
                        items[*selected].1 = !items[*selected].1;
                    }
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    *selected = (*selected + 1) % items.len();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    *selected = (*selected + items.len() - 1) % items.len();
                    self.dialog = Some(dialog);
                    Ok(())
                }
                _ => {
                    self.dialog = Some(dialog);
                    Ok(())
                }
            },
        };
        if result.is_err() && self.dialog.is_none() {
            self.dialog = Some(previous);
        }
        result
    }

    fn submit_input(&mut self, kind: InputKind, value: &str) -> Result<()> {
        match kind {
            InputKind::RouteName | InputKind::RoutePath => self.route_input(kind, value)?,
            InputKind::Command => self.command(value)?,
            InputKind::Open => self.open_project(Path::new(value))?,
            InputKind::NewName => {
                project_name(value)?;
                self.new_draft.name = value.into();
                let path = std::env::current_dir()?.join(value);
                self.input(
                    InputKind::NewOutput,
                    tr!("Directory to create", "Dossier à créer"),
                    &path.to_string_lossy(),
                );
            }
            InputKind::NewOutput => {
                ensure!(
                    !value.is_empty(),
                    trf!(
                        "Enter the directory to create",
                        "Indique le dossier à créer"
                    )
                );
                let path = std::env::current_dir()?.join(value);
                ensure!(
                    path.symlink_metadata()
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
                    trf!(
                        "This directory already exists or cannot be accessed",
                        "Ce dossier existe déjà ou est inaccessible"
                    )
                );
                ensure!(
                    path.parent().is_some_and(Path::is_dir),
                    trf!(
                        "The parent directory must exist",
                        "Le dossier parent doit exister"
                    )
                );
                self.new_draft.output = path;
                self.input(
                    InputKind::NewPort,
                    tr!("Server port", "Port du serveur"),
                    "3000",
                );
            }
            InputKind::NewPort => {
                let port = value.parse::<u16>().context(tr!(
                    "Choose a port between 1 and 65535",
                    "Choisis un port entre 1 et 65535"
                ))?;
                ensure!(
                    port > 0,
                    trf!(
                        "The port must be greater than zero",
                        "Le port doit être supérieur à zéro"
                    )
                );
                self.new_draft.port = port;
                self.choose(
                    ChoiceKind::Database,
                    tr!("Database", "Base de données"),
                    &[
                        tr!(
                            "SQLite — local file, automatic setup",
                            "SQLite — fichier local, configuration automatique"
                        ),
                        tr!(
                            "MySQL — connect to a server",
                            "MySQL — connexion à un serveur"
                        ),
                    ],
                    vec![
                        tr!(
                            "Each model generates a SQL table, a Rust model and CRUD routes.",
                            "Chaque modèle générera sa table SQL, son modèle Rust et son CRUD."
                        )
                        .into(),
                    ],
                    0,
                );
            }
            InputKind::NewLocal => {
                let pack = crate::templates::Pack::local(Path::new(value)).context(tr!(
                    "Invalid local pack: choose the complete template directory",
                    "Pack local invalide : choisis le dossier complet du template"
                ))?;
                ensure!(
                    pack.id == self.new_draft.database.stack(),
                    trf!(
                        "This template uses another database; choose a {} pack",
                        "Ce template utilise une autre base ; choisis un pack {}",
                        self.new_draft.database.name()
                    )
                );
                self.new_draft.location = Path::new(value)
                    .canonicalize()?
                    .to_string_lossy()
                    .into_owned();
                self.new_draft.local_hint = Some(PathBuf::from(&self.new_draft.location));
                self.confirm_new();
            }
            InputKind::NewUrl => {
                let url = reqwest::Url::parse(value).context(tr!("Invalid URL", "URL invalide"))?;
                ensure!(
                    url.scheme() == "https"
                        || (url.scheme() == "http"
                            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))),
                    trf!(
                        "Use HTTPS (or loopback HTTP for tests)",
                        "Utilise HTTPS (ou HTTP sur loopback pour les tests)"
                    )
                );
                self.new_draft.location = value.into();
                self.confirm_new();
            }
            InputKind::ResourceName => {
                let resource = Resource::new(value.into(), Vec::new());
                resource.validate_identity()?;
                ensure!(
                    !self.project.as_ref().is_some_and(|p| p
                        .resources
                        .iter()
                        .any(|r| r.module() == resource.module() || r.table() == resource.table())),
                    trf!(
                        "This model or its table already exists",
                        "Cette ressource ou sa table existe déjà"
                    )
                );
                self.add_draft.name = value.into();
                self.add_draft.path = resource.path;
                self.input(
                    InputKind::FieldName,
                    tr!(
                        "Field name — id is automatic",
                        "Nom du champ — id est automatique"
                    ),
                    "",
                );
            }
            InputKind::FieldName => {
                let field = parse_field(&format!("{value}:string"))?;
                let mut fields = self.add_draft.fields.clone();
                fields.push(field.clone());
                self.validate_draft_fields(&fields)?;
                self.add_draft.pending = Some(field);
                self.choose(
                    ChoiceKind::FieldType,
                    tr!("Field type", "Type du champ"),
                    &[
                        tr!(
                            "Short text (255 characters)",
                            "Texte court (255 caractères)"
                        ),
                        tr!("Long text", "Texte long"),
                        tr!("Integer", "Nombre entier"),
                        tr!("Decimal number", "Nombre décimal"),
                        tr!("Yes / No", "Oui / Non"),
                        tr!(
                            "Foreign key — relationship to a model",
                            "Clé étrangère — relation vers un modèle"
                        ),
                    ],
                    Vec::new(),
                    0,
                );
            }
            InputKind::Collection => {
                let resource = self.draft_resource(value);
                resource.validate()?;
                ensure!(
                    !self
                        .project
                        .as_ref()
                        .is_some_and(|p| p.resources.iter().any(|r| r.path == value)),
                    trf!(
                        "This path is already in use",
                        "Cette adresse est déjà utilisée"
                    )
                );
                self.add_draft.path = value.into();
                let mut description = vec![format!("{} → {}", resource.name, resource.path)];
                description.extend(resource.fields.iter().map(|f| {
                    format!(
                        "{} : {:?}{}{}{}{}",
                        f.name,
                        f.kind,
                        if f.nullable {
                            tr!(" • optional", " • facultatif")
                        } else {
                            ""
                        },
                        if f.unique { " • unique" } else { "" },
                        if f.private {
                            tr!(" • private", " • privé")
                        } else {
                            ""
                        },
                        f.references.as_ref().map_or(String::new(), |r| format!(
                            " → {}.id · {}",
                            r.resource,
                            r.on_delete.sql()
                        ))
                    )
                }));
                description.push(
                    tr!(
                        "Existing files will be preserved.",
                        "Les fichiers existants seront préservés."
                    )
                    .into(),
                );
                description.push(trf!(
                    "SQL table: {} • Rust model + CRUD routes",
                    "Table SQL : {} • modèle Rust + routes CRUD",
                    resource.table()
                ));
                description.push(
                    if self
                        .project
                        .as_ref()
                        .is_some_and(|p| p.database() == Database::Sqlite)
                        && self.api.is_none()
                    {
                        tr!(
                            "SQLite: pending migrations will be applied automatically.",
                            "SQLite : les migrations en attente seront appliquées automatiquement."
                        )
                    } else {
                        tr!(
                            "Apply the migration with b or the next time the API starts.",
                            "Applique la migration avec b ou au prochain lancement de l’API."
                        )
                    }
                    .into(),
                );
                self.choose(
                    ChoiceKind::AddConfirm,
                    tr!("Add this model?", "Ajouter cette ressource ?"),
                    &[tr!("Generate", "Générer"), tr!("Cancel", "Annuler")],
                    description,
                    0,
                );
            }
            InputKind::RelationName => {
                let mut field = self
                    .add_draft
                    .pending
                    .clone()
                    .context(tr!("Missing relationship", "Relation manquante"))?;
                field.name = value.into();
                let mut fields = self.add_draft.fields.clone();
                fields.push(field.clone());
                self.validate_draft_fields(&fields)?;
                self.add_draft.pending = Some(field);
                self.field_options()?;
            }
            InputKind::TemplateLocal => {
                let pack = crate::templates::Pack::local(Path::new(value))?;
                ensure!(
                    self.project.as_ref().is_some_and(|p| p.stack == pack.id),
                    trf!(
                        "Choose a template for the same database engine",
                        "Choisis un template pour la même base de données"
                    )
                );
                ensure!(
                    pack.features.iter().any(|f| f == "relations"),
                    trf!(
                        "This template does not support relationships",
                        "Ce template ne prend pas en charge les relations"
                    )
                );
                self.template_draft.location = Path::new(value)
                    .canonicalize()?
                    .to_string_lossy()
                    .into_owned();
                self.confirm_template_update();
            }
            InputKind::TemplateUrl => {
                let url = reqwest::Url::parse(value)?;
                ensure!(
                    url.scheme() == "https"
                        || (url.scheme() == "http"
                            && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))),
                    trf!("Use HTTPS (or local HTTP)", "Utilise HTTPS (ou HTTP local)")
                );
                self.template_draft.location = value.into();
                self.confirm_template_update();
            }
            InputKind::HttpPath => {
                http::request_url(&self.base_url, value)?;
                self.request_draft.path = value.into();
                if http::needs_body_form(self.project.as_ref(), &self.request_draft) {
                    self.prepare_request()?;
                } else {
                    self.request_draft.body.clear();
                    self.confirm_request()?;
                }
            }
            InputKind::DbHost => {
                ensure!(
                    !value.is_empty(),
                    trf!("Enter the MySQL server", "Indique le serveur MySQL")
                );
                self.db_draft.host = value.into();
                self.input(
                    InputKind::DbPort,
                    tr!("MySQL port", "Port MySQL"),
                    &self.db_draft.port.to_string(),
                );
            }
            InputKind::DbPort => {
                let port = value
                    .parse::<u16>()
                    .context(tr!("Invalid MySQL port", "Port MySQL invalide"))?;
                ensure!(
                    port > 0,
                    trf!(
                        "The port must be greater than zero",
                        "Le port doit être supérieur à zéro"
                    )
                );
                self.db_draft.port = port;
                self.input(
                    InputKind::DbName,
                    tr!("MySQL database name", "Nom de la base MySQL"),
                    &self.db_draft.database.clone(),
                );
            }
            InputKind::DbName => {
                ensure!(
                    !value.is_empty()
                        && value.len() <= 64
                        && value
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
                    trf!(
                        "Invalid database name: use letters, digits and underscores",
                        "Nom de base invalide : lettres, chiffres et underscores"
                    )
                );
                self.db_draft.database = value.into();
                self.input(
                    InputKind::DbUser,
                    tr!("MySQL user", "Utilisateur MySQL"),
                    &self.db_draft.username.clone(),
                );
            }
            InputKind::DbUser => {
                ensure!(
                    !value.is_empty(),
                    trf!("Enter a user", "Indique un utilisateur")
                );
                self.db_draft.username = value.into();
                self.input(
                    InputKind::DbPassword,
                    tr!("MySQL password (hidden)", "Mot de passe MySQL (masqué)"),
                    &self.db_draft.password.clone(),
                );
            }
            InputKind::DbPassword => {
                self.db_draft.password = value.into();
                self.db_draft.url()?;
                self.choose(
                    ChoiceKind::DbSave,
                    tr!(
                        "Save the MySQL connection?",
                        "Enregistrer la connexion MySQL ?"
                    ),
                    &[
                        tr!("Save to .env and test", "Enregistrer dans .env et tester"),
                        tr!("Cancel", "Annuler"),
                    ],
                    vec![
                        trf!(
                            "Server: {}:{}",
                            "Serveur : {}:{}",
                            self.db_draft.host,
                            self.db_draft.port
                        ),
                        trf!(
                            "Database: {} • user: {}",
                            "Base : {} • utilisateur : {}",
                            self.db_draft.database,
                            self.db_draft.username
                        ),
                        tr!(
                            "Password hidden • other .env settings are preserved",
                            "Mot de passe masqué • les autres réglages .env sont conservés"
                        )
                        .into(),
                        tr!(
                            "The MySQL server must be installed and accessible.",
                            "Le serveur MySQL doit être installé et accessible."
                        )
                        .into(),
                    ],
                    0,
                );
            }
        }
        Ok(())
    }

    fn submit_choice(&mut self, kind: ChoiceKind, selected: usize) -> Result<()> {
        match kind {
            ChoiceKind::Language => self.change_language(if selected == 0 {
                crate::i18n::Language::En
            } else {
                crate::i18n::Language::Fr
            })?,
            ChoiceKind::RouteRecovery => {
                if selected == 0 {
                    self.start_template_update()?;
                    self.resume_route_after_template_update = true;
                } else if selected == 1 {
                    self.confirm_route();
                } else {
                    self.notice =
                        tr!("Route creation cancelled", "Création de route annulée").into();
                }
            }
            ChoiceKind::RouteResource
            | ChoiceKind::RouteAction
            | ChoiceKind::RouteMethod
            | ChoiceKind::RouteBody
            | ChoiceKind::RouteConfirm => self.route_choice(kind, selected)?,
            ChoiceKind::Palette => self.command(
                [
                    "new",
                    "open",
                    "add",
                    "run",
                    "stop",
                    "check",
                    "test",
                    "doctor",
                    "edit",
                    "refresh",
                    "theme",
                    "help",
                    "quit",
                    "db",
                    "route",
                    "request",
                    "code",
                    "schema",
                    "templates",
                    "model",
                    "route-new",
                    "language",
                ][selected],
            )?,
            ChoiceKind::EditModel => {
                let resource = self
                    .project
                    .as_ref()
                    .and_then(|p| p.resources.get(selected))
                    .context(tr!("Missing model", "Modèle manquant"))?
                    .clone();
                self.add_draft = AddDraft {
                    name: resource.name.clone(),
                    path: resource.path.clone(),
                    editing: true,
                    ..Default::default()
                };
                let mut description = vec![trf!(
                    "{} → {} · id: primary key",
                    "{} → {} · id : clé primaire",
                    resource.name,
                    resource.table()
                )];
                description.extend(resource.fields.iter().map(Field::argument));
                self.choose(
                    ChoiceKind::EditAction,
                    tr!(
                        "Edit model — existing fields",
                        "Modifier le modèle — champs existants"
                    ),
                    &[
                        tr!("Add an optional field", "Ajouter un champ facultatif"),
                        tr!(
                            "Add an optional relationship",
                            "Ajouter une relation facultative"
                        ),
                        tr!("Cancel", "Annuler"),
                    ],
                    description,
                    0,
                );
            }
            ChoiceKind::EditAction => match selected {
                0 => self.input(
                    InputKind::FieldName,
                    tr!("New optional field name", "Nom du nouveau champ facultatif"),
                    "",
                ),
                1 => self.choose_relation_target()?,
                _ => self.notice = tr!("Update cancelled", "Modification annulée").into(),
            },
            ChoiceKind::EditConfirm => {
                if selected == 0 {
                    self.generate_resource()?;
                } else {
                    self.notice = tr!("Update cancelled", "Modification annulée").into();
                }
            }
            ChoiceKind::HttpMethod => {
                self.request_draft.method =
                    ["GET", "POST", "PATCH", "PUT", "DELETE", "HEAD", "OPTIONS"][selected].into();
                self.request_draft.body = if matches!(selected, 1..=3) {
                    "{}".into()
                } else {
                    String::new()
                };
                self.input(
                    InputKind::HttpPath,
                    tr!(
                        "HTTP path (e.g. /users/1?limit=10)",
                        "Chemin HTTP (ex. /users/1?limit=10)"
                    ),
                    &self.request_draft.path.clone(),
                );
            }
            ChoiceKind::HttpConfirm => {
                if selected == 0 {
                    self.launch_request()?;
                } else {
                    self.notice = tr!(
                        "Request cancelled · nothing sent",
                        "Requête annulée · aucune requête envoyée"
                    )
                    .into();
                }
            }
            ChoiceKind::Database => {
                self.new_draft.database = if selected == 0 {
                    Database::Sqlite
                } else {
                    Database::Mysql
                };
                self.new_draft.local_hint = std::env::var_os("MIBLO_TEMPLATE_DIR")
                    .map(PathBuf::from)
                    .filter(|path| {
                        crate::templates::Pack::local(path)
                            .is_ok_and(|pack| pack.id == self.new_draft.database.stack())
                    })
                    .or_else(|| {
                        crate::templates::discover_local(
                            &std::env::current_dir().ok()?,
                            self.new_draft.database,
                        )
                    });
                self.choose_source();
            }
            ChoiceKind::Source => {
                self.new_draft.source = selected;
                match selected {
                    0 => self.confirm_new(),
                    1 => self.input(
                        InputKind::NewLocal,
                        tr!("Template directory", "Dossier du template"),
                        &self
                            .new_draft
                            .local_hint
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    ),
                    _ => self.input(
                        InputKind::NewUrl,
                        tr!("Template .tar.gz URL", "URL du template .tar.gz"),
                        "",
                    ),
                }
            }
            ChoiceKind::FieldType if selected == 5 => self.choose_relation_target()?,
            ChoiceKind::RelationTarget => {
                let target = self
                    .relation_targets()
                    .get(selected)
                    .context(tr!("Missing target model", "Modèle cible manquant"))?
                    .clone();
                let mut field = self
                    .add_draft
                    .pending
                    .take()
                    .unwrap_or(parse_field(&format!("{}_id:int", target.module()))?);
                field.kind = FieldType::Int;
                field.references = Some(Reference {
                    resource: target.name,
                    on_delete: OnDelete::Restrict,
                });
                let name = field.name.clone();
                self.add_draft.pending = Some(field);
                self.input(
                    InputKind::RelationName,
                    tr!("Foreign key name", "Nom de la clé étrangère"),
                    &name,
                );
            }
            ChoiceKind::RelationDelete => {
                let mut field = self
                    .add_draft
                    .pending
                    .clone()
                    .context(tr!("Missing relationship", "Relation manquante"))?;
                field
                    .references
                    .as_mut()
                    .context(tr!("Missing relationship", "Relation manquante"))?
                    .on_delete =
                    [OnDelete::Restrict, OnDelete::Cascade, OnDelete::SetNull][selected];
                self.store_field(field)?;
            }
            ChoiceKind::TemplateSource => {
                self.template_draft.source = selected;
                match selected {
                    0 => self.confirm_template_update(),
                    1 => {
                        let root = self.ready_for_job()?;
                        let database = self
                            .project
                            .as_ref()
                            .context(tr!("No project", "Aucun projet"))?
                            .database();
                        let path = crate::templates::discover_local(root, database)
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        self.input(
                            InputKind::TemplateLocal,
                            tr!("New template directory", "Dossier du nouveau template"),
                            &path,
                        );
                    }
                    _ => self.input(
                        InputKind::TemplateUrl,
                        tr!(
                            "Template 0.6.0 URL (.tar.gz)",
                            "URL du template 0.6.0 (.tar.gz)"
                        ),
                        "",
                    ),
                }
            }
            ChoiceKind::TemplateConfirm => {
                if selected == 0 {
                    let root = self.ready_for_job()?.as_os_str().to_owned();
                    let mut args =
                        vec!["template".into(), "update".into(), "--project".into(), root];
                    match self.template_draft.source {
                        1 => args.extend([
                            "--template-dir".into(),
                            self.template_draft.location.clone().into(),
                        ]),
                        2 => args.extend([
                            "--template-url".into(),
                            self.template_draft.location.clone().into(),
                        ]),
                        _ => args.extend([
                            "--template-base-url".into(),
                            crate::templates::DEFAULT_BASE.into(),
                        ]),
                    }
                    self.cli_job(args, JobKind::Templates, None, true)?;
                } else {
                    self.resume_route_after_template_update = false;
                }
            }
            ChoiceKind::FieldType => {
                let field = self
                    .add_draft
                    .pending
                    .as_mut()
                    .context(tr!("Missing field", "Champ manquant"))?;
                field.kind = [
                    FieldType::String,
                    FieldType::Text,
                    FieldType::Int,
                    FieldType::Float,
                    FieldType::Bool,
                ][selected];
                self.field_options()?;
            }
            ChoiceKind::NextField => match selected {
                0 => self.input(InputKind::FieldName, tr!("Field name", "Nom du champ"), ""),
                1 if self.add_draft.editing => self.confirm_model_edit()?,
                1 => self.input(
                    InputKind::Collection,
                    tr!("Collection path", "Adresse de la collection"),
                    &self.add_draft.path.clone(),
                ),
                3 => {
                    self.add_draft.pending = None;
                    self.choose_relation_target()?;
                }
                _ => self.notice = tr!("Addition cancelled", "Ajout annulé").into(),
            },
            ChoiceKind::NewConfirm if selected == 0 => self.generate_new()?,
            ChoiceKind::NewRetry => match selected {
                0 => self.choose_source(),
                1 => self.generate_new()?,
                _ => self.notice = tr!("Creation cancelled", "Création annulée").into(),
            },
            ChoiceKind::AddConfirm if selected == 0 => self.generate_resource()?,
            ChoiceKind::NewConfirm | ChoiceKind::AddConfirm => {
                self.notice = tr!("Generation cancelled", "Génération annulée").into()
            }
            ChoiceKind::DbMenu => match selected {
                0 => self.database_action(Action::Check)?,
                1 => self.confirm_database_action(Action::Migrate)?,
                2 => self.confirm_database_action(Action::Init)?,
                4 => self.toggle_schema()?,
                5 => self.start_template_update()?,
                3 if self
                    .project
                    .as_ref()
                    .is_some_and(|p| p.database() == Database::Mysql) =>
                {
                    self.start_database_setup()?
                }
                _ => {}
            },
            ChoiceKind::DbConfirm(action) => {
                if selected == 0 {
                    self.database_action(action)?;
                }
            }
            ChoiceKind::DbSave => {
                if selected == 0 {
                    database::save_mysql(self.ready_for_job()?, &self.db_draft)?;
                    self.db_draft.password.clear();
                    self.database_action(Action::Check)?;
                } else {
                    self.db_draft.password.clear();
                }
            }
        }
        Ok(())
    }
    fn submit_options(
        &mut self,
        items: &[(String, bool)],
        unique_available: bool,
        private_available: bool,
    ) -> Result<()> {
        let mut field = self
            .add_draft
            .pending
            .clone()
            .context(tr!("Missing field", "Champ manquant"))?;
        field.nullable = self.add_draft.editing || items[0].1;
        let mut index = 1;
        if unique_available {
            field.unique = items[index].1;
            index += 1;
        }
        if private_available {
            field.private = items[index].1;
        }
        if field.references.is_some() {
            let mut choices = vec![
                tr!(
                    "Block deletion if other rows reference it (RESTRICT)",
                    "Bloquer la suppression si des lignes font référence (RESTRICT)"
                ),
                tr!(
                    "Also delete related rows (CASCADE)",
                    "Supprimer aussi les lignes liées (CASCADE)"
                ),
            ];
            if field.nullable {
                choices.push(tr!(
                    "Keep rows and clear the relationship (SET NULL)",
                    "Garder les lignes et vider le lien (SET NULL)"
                ));
            }
            self.add_draft.pending = Some(field);
            self.choose(
                ChoiceKind::RelationDelete,
                tr!(
                    "When the referenced row is deleted…",
                    "Si la ligne référencée est supprimée…"
                ),
                &choices,
                vec![
                    tr!(
                        "The database enforces this rule automatically.",
                        "La base appliquera cette règle automatiquement."
                    )
                    .into(),
                    tr!(
                        "CASCADE deletes the related rows.",
                        "CASCADE supprime réellement les lignes liées."
                    )
                    .into(),
                ],
                0,
            );
            return Ok(());
        }
        self.store_field(field)
    }
    fn store_field(&mut self, field: Field) -> Result<()> {
        let mut fields = self.add_draft.fields.clone();
        fields.push(field);
        self.validate_draft_fields(&fields)?;
        self.add_draft.fields = fields;
        self.add_draft.pending = None;
        self.choose(
            ChoiceKind::NextField,
            tr!("Model fields", "Champs de la ressource"),
            &[
                tr!("Add another field", "Ajouter un autre champ"),
                tr!("Finish and generate", "Terminer et générer"),
                tr!("Cancel", "Annuler"),
                tr!(
                    "Add a relationship (foreign key)",
                    "Ajouter une relation (clé étrangère)"
                ),
            ],
            vec![trf!(
                "{} field(s) defined",
                "{} champ(s) définis",
                self.add_draft.fields.len()
            )],
            0,
        );
        Ok(())
    }
    fn field_options(&mut self) -> Result<()> {
        let field = self
            .add_draft
            .pending
            .as_ref()
            .context(tr!("Missing field", "Champ manquant"))?;
        let unique_available = field.kind != FieldType::Text;
        let private_available = !field.private;
        let mut items = vec![(
            if self.add_draft.editing {
                tr!(
                    "Optional — existing rows keep NULL (fixed)",
                    "Facultatif — anciennes lignes à NULL (fixé)"
                )
            } else {
                tr!("Optional — allow null", "Facultatif — autoriser null")
            }
            .into(),
            self.add_draft.editing,
        )];
        if unique_available {
            items.push((
                if field.references.is_some() {
                    tr!(
                        "Unique — one row per reference (one-to-one)",
                        "Unique — une seule ligne par référence (1 à 1)"
                    )
                } else {
                    tr!(
                        "Unique — prevent duplicates",
                        "Unique — interdire les doublons"
                    )
                }
                .into(),
                false,
            ));
        }
        if private_available {
            items.push((
                tr!(
                    "Private — hide in responses",
                    "Privé — masquer dans les réponses"
                )
                .into(),
                false,
            ));
        } else {
            self.notice = tr!(
                "This sensitive field will be hidden in responses",
                "Ce champ sensible sera masqué dans les réponses"
            )
            .into();
        }
        self.dialog = Some(Dialog::Options {
            items,
            selected: 0,
            unique_available,
            private_available,
        });
        Ok(())
    }
    fn relation_targets(&self) -> Vec<Resource> {
        let mut models = self
            .project
            .as_ref()
            .map_or(vec![], |p| p.resources.clone());
        if !self.add_draft.editing {
            models.push(Resource::new(self.add_draft.name.clone(), vec![]));
        }
        models
    }
    fn choose_relation_target(&mut self) -> Result<()> {
        ensure!(
            !self.add_draft.name.is_empty(),
            trf!("Give the model a name", "Donne un nom au modèle")
        );
        let labels: Vec<_> = self
            .relation_targets()
            .iter()
            .map(|r| {
                format!(
                    "{} → {}.id{}",
                    r.name,
                    r.table(),
                    if r.name == self.add_draft.name {
                        tr!(" (this model)", " (ce modèle)")
                    } else {
                        ""
                    }
                )
            })
            .collect();
        let items: Vec<_> = labels.iter().map(String::as_str).collect();
        self.choose(
            ChoiceKind::RelationTarget,
            tr!("Model / table to reference", "Modèle / table à référencer"),
            &items,
            vec![
                tr!(
                    "Every table has an automatic id primary key.",
                    "La clé primaire id est automatique dans chaque table."
                )
                .into(),
                tr!(
                    "A foreign key requires the referenced row to exist.",
                    "Une clé étrangère impose l’existence de la ligne référencée."
                )
                .into(),
            ],
            0,
        );
        Ok(())
    }
    pub(super) fn start_template_update(&mut self) -> Result<()> {
        let (_, state) = generation::inspect(self.ready_for_job()?)?;
        self.resume_route_after_template_update = false;
        self.template_draft = TemplateDraft::default();
        self.choose(
            ChoiceKind::TemplateSource,
            tr!(
                "Templates for new models and routes",
                "Templates pour les futurs modèles et routes"
            ),
            &[
                tr!(
                    "GitHub Releases — version 0.6.0, model CRUD routes",
                    "GitHub Releases — version 0.6.0, routes CRUD de modèles"
                ),
                tr!("Local directory", "Dossier local"),
                tr!("Custom URL", "URL personnalisée"),
            ],
            vec![
                trf!(
                    "Current version: {}",
                    "Version actuelle : {}",
                    state.template.version
                ),
                tr!(
                    "Existing files and applied migrations are preserved.",
                    "Les fichiers existants et migrations appliquées sont conservés."
                )
                .into(),
            ],
            0,
        );
        Ok(())
    }
    fn confirm_template_update(&mut self) {
        let source = if self.template_draft.source == 0 {
            "GitHub Releases · version 0.6.0".into()
        } else {
            self.template_draft.location.clone()
        };
        self.choose(
            ChoiceKind::TemplateConfirm,
            tr!(
                "Use this template for new models and routes?",
                "Utiliser ce template pour les futurs modèles et routes ?"
            ),
            &[tr!("Update", "Mettre à jour"), tr!("Cancel", "Annuler")],
            vec![
                source,
                tr!(
                    "The template configuration will be updated.",
                    "La configuration du template sera mise à jour."
                )
                .into(),
                tr!(
                    "Existing code and applied migrations are preserved.",
                    "Le code existant et les migrations appliquées sont conservés."
                )
                .into(),
            ],
            0,
        );
    }
    fn draft_resource(&self, path: &str) -> Resource {
        Resource {
            name: self.add_draft.name.clone(),
            path: path.into(),
            fields: self.add_draft.fields.clone(),
        }
    }
    fn confirm_new(&mut self) {
        self.choose(
            ChoiceKind::NewConfirm,
            tr!("Create this API?", "Créer cette API ?"),
            &[
                tr!("Create project", "Créer le projet"),
                tr!("Cancel", "Annuler"),
            ],
            vec![
                self.new_draft.name.clone(),
                self.new_draft.output.display().to_string(),
                format!("Rust / Axum / {}", self.new_draft.database.name()),
                trf!("Port: {}", "Port : {}", self.new_draft.port),
                match self.new_draft.source {
                    1 => trf!(
                        "Local template: {}",
                        "Template local : {}",
                        self.new_draft.location
                    ),
                    2 => trf!(
                        "Remote template: {}",
                        "Template distant : {}",
                        self.new_draft.location
                    ),
                    _ => tr!(
                        "Template: GitHub Releases (published archive required)",
                        "Template : GitHub Releases (archive publiée requise)"
                    )
                    .into(),
                },
                tr!(
                    "You can add models from the workspace.",
                    "Tu pourras ajouter tes ressources depuis l’atelier."
                )
                .into(),
            ],
            0,
        );
    }
    fn generate_new(&mut self) -> Result<()> {
        let draft = &self.new_draft;
        let mut args: Vec<OsString> = vec![
            "new".into(),
            draft.name.clone().into(),
            "--output".into(),
            draft.output.as_os_str().to_owned(),
            "--port".into(),
            draft.port.to_string().into(),
            "--yes".into(),
            "--database".into(),
            draft.database.arg().into(),
        ];
        if draft.source == 1 {
            args.extend(["--template-dir".into(), draft.location.clone().into()]);
        }
        if draft.source == 2 {
            args.extend(["--template-url".into(), draft.location.clone().into()]);
        }
        self.cli_job(args, JobKind::Create, Some(draft.output.clone()), true)
    }
    fn generate_resource(&mut self) -> Result<()> {
        let root = self.ready_for_job()?.as_os_str().to_owned();
        let mut args: Vec<OsString> = vec![
            if self.add_draft.editing {
                "edit"
            } else {
                "add"
            }
            .into(),
            self.add_draft.name.clone().into(),
            "--project".into(),
            root,
            "--yes".into(),
        ];
        if !self.add_draft.editing {
            args.extend(["--path".into(), self.add_draft.path.clone().into()]);
        } else {
            ensure!(
                self.api.is_none(),
                trf!(
                    "Stop the API before editing a model",
                    "Arrête l’API avant de modifier un modèle"
                )
            );
        }
        for field in &self.add_draft.fields {
            args.push("--field".into());
            args.push(field.argument().into());
        }
        self.cli_job(
            args,
            if self.add_draft.editing {
                JobKind::Edit
            } else {
                JobKind::Add
            },
            None,
            false,
        )?;
        self.migrate_after_add = !self.add_draft.editing;
        Ok(())
    }

    pub(super) fn database_menu(&mut self) -> Result<()> {
        let target = database::target_label(self.ready_for_job()?)
            .unwrap_or_else(|_| tr!("Connection needs setup", "Connexion à configurer").into());
        let mysql = self
            .project
            .as_ref()
            .is_some_and(|p| p.database() == Database::Mysql);
        self.choose(
            ChoiceKind::DbMenu,
            tr!("Database", "Base de données"),
            &[
                tr!("Test connection", "Tester la connexion"),
                tr!("Apply migrations", "Appliquer les migrations"),
                tr!(
                    "Initialize the database and tables",
                    "Initialiser la base et ses tables"
                ),
                if mysql {
                    tr!(
                        "Configure MySQL connection",
                        "Configurer la connexion MySQL"
                    )
                } else {
                    tr!("Close", "Fermer")
                },
                tr!("View database schema", "Voir le schéma de la base"),
                tr!("Update templates", "Mettre à jour les templates"),
            ],
            vec![
                if mysql {
                    tr!(
                        "MySQL • configure a connection to an accessible server",
                        "MySQL • connexion à configurer avec un serveur accessible"
                    )
                } else {
                    tr!(
                        "SQLite • local file, automatic setup",
                        "SQLite • fichier local, configuration automatique"
                    )
                }
                .into(),
                target,
                self.db_status.label().into(),
            ],
            0,
        );
        Ok(())
    }

    pub(super) fn start_database_setup(&mut self) -> Result<()> {
        ensure!(
            self.api.is_none(),
            trf!(
                "Stop the API before changing its connection",
                "Arrête l’API avant de modifier sa connexion"
            )
        );
        let root = self.ready_for_job()?;
        if self
            .project
            .as_ref()
            .is_some_and(|p| p.database() == Database::Sqlite)
        {
            return self.database_menu();
        }
        self.db_draft = database::MysqlSettings::load(root);
        self.input(
            InputKind::DbHost,
            tr!("MySQL server", "Serveur MySQL"),
            &self.db_draft.host.clone(),
        );
        Ok(())
    }

    pub(super) fn confirm_database_action(&mut self, action: Action) -> Result<()> {
        let target = database::target_label(self.ready_for_job()?)?;
        self.choose(
            ChoiceKind::DbConfirm(action),
            tr!("Apply SQL changes?", "Appliquer les changements SQL ?"),
            &[tr!("Confirm", "Confirmer"), tr!("Cancel", "Annuler")],
            vec![
                target,
                if action == Action::Init {
                    tr!(
                        "The database will be created if missing, then migrations applied.",
                        "La base sera créée si elle n’existe pas, puis les migrations appliquées."
                    )
                } else {
                    tr!(
                        "Pending SQL migrations will be applied to the configured database.",
                        "Les migrations SQL en attente seront appliquées à la base configurée."
                    )
                }
                .into(),
                tr!(
                    "Migrations already applied are not run again.",
                    "Les migrations déjà appliquées ne sont pas rejouées."
                )
                .into(),
            ],
            0,
        );
        Ok(())
    }

    pub(super) fn database_action(&mut self, action: Action) -> Result<()> {
        ensure!(
            action == Action::Check || self.api.is_none(),
            trf!(
                "Stop the API before applying migrations",
                "Arrête l’API avant d’appliquer les migrations"
            )
        );
        let root = self.ready_for_job()?.as_os_str().to_owned();
        let arg = match action {
            Action::Schema => return self.toggle_schema(),
            Action::Check => "check",
            Action::Migrate => "migrate",
            Action::Init => "init",
        };
        self.cli_job(
            vec!["db".into(), arg.into(), "--project".into(), root],
            JobKind::Database,
            None,
            false,
        )?;
        self.db_job = Some(action);
        self.db_status = status::DatabaseNotice::Working;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_wait_for_confirmation_and_cancel_sends_nothing() {
        let mut app = App::new();
        app.base_url = "http://127.0.0.1:1".into();
        for method in ["POST", "PATCH", "PUT", "DELETE"] {
            app.request_draft = http::Request {
                method: method.into(),
                path: "/users/1".into(),
                body: "{}".into(),
            };
            app.confirm_request().unwrap();
            assert!(matches!(
                app.dialog,
                Some(Dialog::Select {
                    kind: ChoiceKind::HttpConfirm,
                    selected: 1,
                    ..
                })
            ));
            assert!(!app.tester.pending);
            app.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .unwrap();
            assert!(app.dialog.is_none() && !app.tester.pending && app.tester.reply.is_none());
        }
        app.request_draft.body = "broken JSON".into();
        assert!(app.confirm_request().is_err());
        assert!(!app.tester.pending);
    }

    #[test]
    fn resource_form_validates_and_builds_fields_without_writing() {
        let mut app = App::new();
        app.project = Some(Project::new("sample".into(), 3000, Vec::new()));
        app.submit_input(InputKind::ResourceName, "Product")
            .unwrap();
        assert!(app.submit_input(InputKind::FieldName, "id").is_err());
        app.submit_input(InputKind::FieldName, "title").unwrap();
        app.submit_choice(ChoiceKind::FieldType, 0).unwrap();
        app.submit_options(
            &[
                ("nullable".into(), false),
                ("unique".into(), true),
                ("private".into(), false),
            ],
            true,
            true,
        )
        .unwrap();
        assert!(app.add_draft.fields[0].unique);
        assert!(!app.add_draft.fields[0].nullable);
        app.submit_input(InputKind::FieldName, "token").unwrap();
        app.submit_choice(ChoiceKind::FieldType, 1).unwrap();
        app.submit_options(&[("nullable".into(), true)], false, false)
            .unwrap();
        assert!(app.add_draft.fields[1].nullable && app.add_draft.fields[1].private);
        assert!(app.submit_input(InputKind::Collection, "/health").is_err());
        app.submit_input(InputKind::Collection, "/products")
            .unwrap();
        assert!(matches!(
            app.dialog,
            Some(Dialog::Select {
                kind: ChoiceKind::AddConfirm,
                ..
            })
        ));
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        assert!(app.dialog.is_none() && app.job.is_none());
    }
}
