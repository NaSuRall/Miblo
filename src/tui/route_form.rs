use super::*;
use crate::config::{CustomRoute, HTTP_METHODS, RouteAction};
use forms::{ChoiceKind, InputKind};
use heck::ToSnakeCase;

impl App {
    pub(super) fn start_route_create(&mut self) -> Result<()> {
        self.ready_for_job()?;
        self.route_draft = CustomRoute {
            method: "GET".into(),
            ..Default::default()
        };
        let mut items = vec![
            tr!(
                "Custom logic — no model",
                "Logique personnalisée — sans modèle"
            )
            .to_owned(),
        ];
        items.extend(
            self.project
                .as_ref()
                .context(tr!("Open a project first", "Ouvre un projet d’abord"))?
                .resources
                .iter()
                .map(|r| format!("{} · {}", r.name, r.path)),
        );
        self.choose(
            ChoiceKind::RouteResource,
            tr!(
                "Create a route — linked model",
                "Créer une route — modèle associé"
            ),
            &items.iter().map(String::as_str).collect::<Vec<_>>(),
            vec![
                tr!(
                    "Choose a model, then an action: Miblo connects its CRUD and SQL queries.",
                    "Choisis un modèle, puis l’action : Miblo branche son CRUD et ses requêtes SQL."
                )
                .into(),
                tr!(
                    "The model's fields, validation and relationships are reused.",
                    "Les champs, validations et relations du modèle sont réutilisés."
                )
                .into(),
                tr!(
                    "Model missing? Escape, then a to create its table and fields.",
                    "Modèle absent ? Échap puis a pour créer sa table et ses champs."
                )
                .into(),
            ],
            usize::from(items.len() > 1),
        );
        Ok(())
    }

    pub(super) fn route_input(&mut self, kind: InputKind, value: &str) -> Result<()> {
        match kind {
            InputKind::RoutePath => {
                let mut route = self.route_draft.clone();
                route.path = value.into();
                route.name = "custom_route".into();
                route.validate()?;
                self.route_draft.path = value.into();
                let suggested = format!(
                    "{}_{}",
                    self.route_draft.method.to_lowercase(),
                    value.to_snake_case()
                );
                self.input(
                    InputKind::RouteName,
                    tr!(
                        "Route / function name (snake_case)",
                        "Nom de la route / fonction (snake_case)"
                    ),
                    &suggested,
                );
            }
            InputKind::RouteName => {
                self.route_draft.name = value.into();
                self.validate_route_draft()?;
                if self.route_draft.action.is_none()
                    && ["POST", "PUT", "PATCH", "DELETE"]
                        .contains(&self.route_draft.method.as_str())
                {
                    self.choose(
                        ChoiceKind::RouteBody,
                        tr!("Request body", "Corps de la requête"),
                        &["JSON", tr!("No body", "Sans corps")],
                        vec![
                            tr!(
                                "The handler receives JSON and passes it to the business function.",
                                "JSON sera reçu dans le handler et transmis à la fonction métier."
                            )
                            .into(),
                        ],
                        0,
                    );
                } else {
                    self.confirm_route();
                }
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    pub(super) fn route_choice(&mut self, kind: ChoiceKind, selected: usize) -> Result<()> {
        match kind {
            ChoiceKind::RouteResource => {
                if selected > 0 {
                    let resource = self
                        .project
                        .as_ref()
                        .and_then(|p| p.resources.get(selected - 1))
                        .context(tr!("Model not found", "Modèle introuvable"))?;
                    self.route_draft.resource = Some(resource.name.clone());
                    self.route_draft.path = format!("{}/custom", resource.path);
                    let mut actions: Vec<_> = RouteAction::ALL.iter().map(|a| a.label()).collect();
                    actions.push(tr!(
                        "Custom logic — to implement (501)",
                        "Logique personnalisée — à compléter (501)"
                    ));
                    self.choose(
                        ChoiceKind::RouteAction,
                        tr!(
                            "Route action for this model",
                            "Action de la route pour ce modèle"
                        ),
                        &actions,
                        vec![
                            tr!(
                                "CRUD uses the existing table. No new migration.",
                                "Le CRUD utilise la table existante. Aucune nouvelle migration."
                            )
                            .into(),
                        ],
                        0,
                    );
                    return Ok(());
                } else {
                    self.route_draft.resource = None;
                    self.route_draft.action = None;
                    self.route_draft.path = "/custom".into();
                }
                self.choose(
                    ChoiceKind::RouteMethod,
                    tr!("New route method", "Méthode de la nouvelle route"),
                    HTTP_METHODS,
                    Vec::new(),
                    0,
                );
            }
            ChoiceKind::RouteAction => {
                self.route_draft.action = RouteAction::ALL.get(selected).copied();
                if let Some(action) = self.route_draft.action {
                    self.route_draft.method = action.method().into();
                    self.route_draft.json_body = action.json_body();
                    let resource = self
                        .project
                        .as_ref()
                        .and_then(|p| {
                            p.resources
                                .iter()
                                .find(|r| Some(&r.name) == self.route_draft.resource.as_ref())
                        })
                        .context(tr!("Model not found", "Modèle introuvable"))?;
                    self.route_draft.path = format!(
                        "{}/{}{}",
                        resource.path,
                        action.arg(),
                        if action.needs_id() { "/{id}" } else { "" }
                    );
                    self.input(
                        InputKind::RoutePath,
                        tr!("Route path", "Chemin de la route"),
                        &self.route_draft.path.clone(),
                    );
                } else {
                    self.choose(
                        ChoiceKind::RouteMethod,
                        tr!("New route method", "Méthode de la nouvelle route"),
                        HTTP_METHODS,
                        vec![
                            tr!(
                                "Complete the business function (initial response 501).",
                                "La fonction métier sera à compléter (réponse initiale 501)."
                            )
                            .into(),
                        ],
                        0,
                    );
                }
            }
            ChoiceKind::RouteMethod => {
                self.route_draft.method = HTTP_METHODS[selected].into();
                self.input(
                    InputKind::RoutePath,
                    tr!(
                        "Route path (e.g. /users/{id}/activate)",
                        "Chemin de la route (ex. /users/{id}/activate)"
                    ),
                    &self.route_draft.path.clone(),
                );
            }
            ChoiceKind::RouteBody => {
                self.route_draft.json_body = selected == 0;
                self.validate_route_draft()?;
                self.confirm_route();
            }
            ChoiceKind::RouteConfirm if selected == 0 => {
                self.validate_route_draft()?;
                let root = self.ready_for_job()?.as_os_str().to_owned();
                let route = &self.route_draft;
                let mut args: Vec<OsString> = vec![
                    "route".into(),
                    "add".into(),
                    route.name.clone().into(),
                    "--method".into(),
                    route.method.clone().into(),
                    "--path".into(),
                    route.path.clone().into(),
                    "--project".into(),
                    root,
                    "--yes".into(),
                ];
                if let Some(resource) = &route.resource {
                    args.extend(["--resource".into(), resource.into()]);
                }
                if route.json_body {
                    args.push("--json-body".into());
                }
                if let Some(action) = route.action {
                    args.extend(["--action".into(), action.arg().into()]);
                }
                self.cli_job(args, JobKind::Route, None, false)?;
            }
            ChoiceKind::RouteConfirm => {
                self.notice = tr!("Route creation cancelled", "Création de route annulée").into()
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn validate_route_draft(&self) -> Result<()> {
        let mut project = self
            .project
            .clone()
            .context(tr!("Open a project first", "Ouvre un projet d’abord"))?;
        project.routes.push(self.route_draft.clone());
        project.validate()
    }

    pub(super) fn confirm_route(&mut self) {
        let route = &self.route_draft;
        let mut description = vec![
            format!("{} {}", route.method, route.path),
            trf!(
                "Model: {} · body: {}",
                "Modèle : {} · corps : {}",
                route
                    .resource
                    .as_deref()
                    .unwrap_or(tr!("standalone", "indépendante")),
                if route.json_body && route.action.is_some() {
                    tr!("Model JSON", "JSON du modèle")
                } else if route.json_body {
                    "JSON"
                } else {
                    "aucun"
                }
            ),
            trf!(
                "Directory: src/resources/custom_routes/{}/",
                "Dossier : src/resources/custom_routes/{}/",
                route.name
            ),
        ];
        if let Some(action) = route.action {
            description.extend([
                trf!(
                    "Action: {} · model SQL connected automatically",
                    "Action : {} · SQL du modèle branché automatiquement",
                    action.label()
                ),
                tr!(
                    "The handler reuses the existing CRUD and validation.",
                    "Le handler réutilise le CRUD existant et ses validations."
                )
                .into(),
                tr!(
                    "The URL name does not add filters or authentication.",
                    "Le nom de l’URL n’ajoute aucun filtre ni authentification."
                )
                .into(),
            ]);
        } else {
            description.extend([
                tr!(
                    "handlers.rs: HTTP · requests.rs: parameters · service.rs: business logic",
                    "handlers.rs : HTTP · requests.rs : paramètres · service.rs : métier"
                )
                .into(),
                tr!(
                    "Initial response: 501 not implemented. Complete service.rs in your editor.",
                    "Réponse initiale : 501 non implémenté. Complète service.rs dans ton éditeur."
                )
                .into(),
            ]);
        }
        description.push(
            tr!(
                "No new table. Restart the API to load the route.",
                "Aucune nouvelle table. Redémarre l’API pour charger la route."
            )
            .into(),
        );
        self.choose(
            ChoiceKind::RouteConfirm,
            tr!("Create this route?", "Créer cette route ?"),
            &[
                tr!("Create files", "Créer les fichiers"),
                tr!("Cancel", "Annuler"),
            ],
            description,
            0,
        );
    }

    pub(super) fn preview_route(&mut self) -> Result<()> {
        let request = self
            .routes
            .get(self.route_selected)
            .context(tr!("Select a route", "Sélectionne une route"))?;
        let project = self
            .project
            .as_ref()
            .context(tr!("Open a project first", "Ouvre un projet d’abord"))?;
        let path = if let Some(route) = project
            .routes
            .iter()
            .find(|r| r.method == request.method && r.path == request.path)
        {
            format!(
                "src/resources/custom_routes/{}/{}.rs",
                route.name,
                if route.action.is_some() {
                    "handlers"
                } else {
                    "service"
                }
            )
        } else if let Some(resource) = project
            .resources
            .iter()
            .find(|r| request.path == r.path || request.path == format!("{}/{{id}}", r.path))
        {
            format!("src/resources/{}/handlers.rs", resource.module())
        } else {
            "src/lib.rs".into()
        };
        self.code_scroll = 0;
        self.code_horizontal = 0;
        self.load_file(Path::new(&path))
    }

    pub(super) fn unimplemented_route(&self) -> Option<&CustomRoute> {
        let reply = self.tester.reply.as_ref()?;
        if reply.status != Some(501) {
            return None;
        }
        http::custom_route(self.project.as_ref()?, &reply.method, &reply.path)
            .filter(|route| route.action.is_none())
    }

    pub(super) fn received_response(&mut self) {
        let Some(reply) = &self.tester.reply else {
            return;
        };
        self.notice = format!(
            "{} : {} · {} ms",
            reply.method,
            reply
                .status
                .map_or(tr!("network failure", "échec réseau").into(), |s| format!(
                    "HTTP {s}"
                )),
            reply.elapsed_ms
        );
        if let Some(route) = self.unimplemented_route() {
            let mut guidance = vec![
                tr!(
                    "Route exists · action to implement (501).",
                    "Route présente · action à implémenter (501)."
                )
                .into(),
                tr!(
                    "e: open its business function in your editor.",
                    "e : ouvrir sa fonction métier dans l’éditeur."
                )
                .into(),
                trf!(
                    "Directory: custom_routes/{}/",
                    "Dossier : custom_routes/{}/",
                    route.name
                ),
                tr!(
                    "File: service.rs · restart after editing.",
                    "Fichier : service.rs · redémarrer après modification."
                )
                .into(),
                tr!(
                    "Creating a table does not implement this function.",
                    "Créer une table ne complète pas cette fonction."
                )
                .into(),
            ];
            if let Some(resource) = route.resource.as_ref().and_then(|name| {
                self.project
                    .as_ref()?
                    .resources
                    .iter()
                    .find(|r| &r.name == name)
            }) {
                guidance.push(trf!(
                    "Existing CRUD: GET / POST {}",
                    "CRUD déjà généré : GET / POST {}",
                    resource.path
                ));
            } else {
                guidance.push(
                    tr!(
                        "a: create a model / table with CRUD.",
                        "a : créer un modèle / table avec son CRUD."
                    )
                    .into(),
                );
            }
            guidance.extend([
                String::new(),
                tr!(
                    "Response received from the API:",
                    "Réponse reçue de l’API :"
                )
                .into(),
            ]);
            guidance.append(&mut self.tester.lines);
            self.tester.lines = guidance;
            self.notice = tr!(
                "HTTP 501 • route exists, business logic to implement • e: open service.rs",
                "HTTP 501 • route présente, métier à compléter • e : ouvrir service.rs"
            )
            .into();
        }
        self.log(self.notice.clone());
    }

    pub(super) fn route_creation_failed(&mut self) {
        let Some(error) = self
            .logs
            .iter()
            .rev()
            .find_map(|line| line.strip_prefix("[route] Error: "))
        else {
            return;
        };
        if ![
            "Le template a changé",
            "Template changed",
            "Ce template ne permet pas",
            "This template does not support",
        ]
        .iter()
        .any(|message| error.contains(message))
        {
            return;
        }
        self.choose(
            ChoiceKind::RouteRecovery,
            tr!("Template update required", "Template à mettre à jour"),
            &[tr!("Update templates", "Mettre à jour les templates"), tr!("Retry route", "Réessayer la route"), tr!("Cancel", "Annuler")],
            vec![
                tr!("Your route is preserved in the form.", "Ta route est conservée dans le formulaire.").into(),
                tr!("Update templates to create new routes.", "Mets à jour les templates pour créer de nouvelles routes.").into(),
                tr!("Existing code and migrations are preserved.", "Le code et les migrations existants sont conservés.").into(),
                tr!("Older models keep their template: its exact version is still needed to edit them.", "Les anciens modèles gardent leur template : sa version exacte reste nécessaire pour les modifier.").into(),
            ],
            0,
        );
    }

    pub(super) fn template_update_finished(&mut self, success: bool) {
        if std::mem::take(&mut self.resume_route_after_template_update) && success {
            self.confirm_route();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Resource, parse_field};
    use forms::Dialog;

    fn app() -> App {
        let mut app = App::new();
        app.project = Some(Project::new(
            "sample".into(),
            3000,
            vec![Resource::new(
                "User".into(),
                vec![parse_field("name:string").unwrap()],
            )],
        ));
        app
    }

    #[test]
    fn choosing_a_model_action_infers_the_method_body_and_confirmation() {
        let mut app = app();
        app.route_choice(ChoiceKind::RouteResource, 1).unwrap();
        assert!(matches!(
            app.dialog,
            Some(Dialog::Select {
                kind: ChoiceKind::RouteAction,
                ..
            })
        ));
        app.route_choice(ChoiceKind::RouteAction, 2).unwrap();
        assert_eq!(app.route_draft.method, "POST");
        assert!(app.route_draft.json_body);
        assert_eq!(app.route_draft.action, Some(RouteAction::Create));
        app.route_input(InputKind::RoutePath, "/members").unwrap();
        app.route_input(InputKind::RouteName, "create_member")
            .unwrap();
        assert!(matches!(
            app.dialog,
            Some(Dialog::Select {
                kind: ChoiceKind::RouteConfirm,
                ..
            })
        ));
        assert!(app.job.is_none());
    }

    #[test]
    fn template_recovery_preserves_the_route_and_cancel_prevents_resuming() {
        let mut app = app();
        app.route_draft = CustomRoute {
            name: "members".into(),
            path: "/members".into(),
            method: "GET".into(),
            ..Default::default()
        };
        app.logs
            .push_back("[route] Error: Le template a changé depuis la création du projet.".into());
        app.route_creation_failed();
        assert!(matches!(
            app.dialog,
            Some(Dialog::Select {
                kind: ChoiceKind::RouteRecovery,
                ..
            })
        ));
        app.resume_route_after_template_update = true;
        app.template_update_finished(true);
        assert!(matches!(
            app.dialog,
            Some(Dialog::Select {
                kind: ChoiceKind::RouteConfirm,
                ..
            })
        ));
        assert_eq!(app.route_draft.path, "/members");
        app.resume_route_after_template_update = true;
        app.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .unwrap();
        app.template_update_finished(true);
        assert!(app.dialog.is_none() && app.job.is_none());
    }

    #[test]
    fn stub_response_explains_the_actual_route_and_opens_its_service() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app();
        let route = CustomRoute {
            name: "activate".into(),
            method: "POST".into(),
            path: "/users/{id}/activate".into(),
            resource: Some("User".into()),
            ..Default::default()
        };
        app.project.as_mut().unwrap().routes.push(route);
        let path = "src/resources/custom_routes/activate/service.rs";
        fs::create_dir_all(dir.path().join(path).parent().unwrap()).unwrap();
        fs::write(dir.path().join(path), "// user's business logic\n").unwrap();
        app.root = Some(dir.path().canonicalize().unwrap());
        app.tester.reply = Some(http::Reply {
            method: "POST".into(),
            path: "/users/42/activate?mode=fast".into(),
            status: Some(501),
            elapsed_ms: 2,
            content_type: "application/json".into(),
            body: "server reply".into(),
            truncated: false,
        });
        app.tester.lines = vec!["server reply".into()];
        app.received_response();
        assert!(
            app.tester
                .lines
                .iter()
                .any(|s| s.contains(tr!("action to implement", "action à implémenter")))
        );
        assert_eq!(app.tester.reply.as_ref().unwrap().body, "server reply");
        app.focus = Focus::Response;
        app.request_editor().unwrap();
        assert!(app.editor_requested);
        assert_eq!(app.file, Some(PathBuf::from(path)));
        for (status, method, path) in [
            (404, "POST", "/users/42/activate"),
            (501, "GET", "/users/42/activate"),
            (501, "POST", "/users"),
        ] {
            let reply = app.tester.reply.as_mut().unwrap();
            reply.status = Some(status);
            reply.method = method.into();
            reply.path = path.into();
            assert!(app.unimplemented_route().is_none());
        }
    }
}
