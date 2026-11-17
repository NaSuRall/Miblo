use super::*;
use crate::config::{CustomRoute, HTTP_METHODS};
use forms::{ChoiceKind, InputKind};
use heck::ToSnakeCase;

impl App {
    pub(super) fn start_route_create(&mut self) -> Result<()> {
        self.ready_for_job()?;
        self.route_draft = CustomRoute {
            method: "GET".into(),
            ..Default::default()
        };
        let mut items = vec!["Route indépendante".to_owned()];
        items.extend(
            self.project
                .as_ref()
                .context("Ouvre un projet d’abord")?
                .resources
                .iter()
                .map(|r| format!("{} · {}", r.name, r.path)),
        );
        self.choose(
            ChoiceKind::RouteResource,
            "Créer une route — modèle associé",
            &items.iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["Un handler HTTP et une fonction métier à compléter dans ton éditeur.".into()],
            0,
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
                    "Nom de la route / fonction (snake_case)",
                    &suggested,
                );
            }
            InputKind::RouteName => {
                self.route_draft.name = value.into();
                self.validate_route_draft()?;
                if ["POST", "PUT", "PATCH", "DELETE"].contains(&self.route_draft.method.as_str()) {
                    self.choose(
                        ChoiceKind::RouteBody,
                        "Corps de la requête",
                        &["JSON", "Sans corps"],
                        vec![
                            "JSON sera reçu dans le handler et transmis à la fonction métier."
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
                        .context("Modèle introuvable")?;
                    self.route_draft.resource = Some(resource.name.clone());
                    self.route_draft.path = format!("{}/custom", resource.path);
                } else {
                    self.route_draft.path = "/custom".into();
                }
                self.choose(
                    ChoiceKind::RouteMethod,
                    "Méthode de la nouvelle route",
                    HTTP_METHODS,
                    Vec::new(),
                    0,
                );
            }
            ChoiceKind::RouteMethod => {
                self.route_draft.method = HTTP_METHODS[selected].into();
                self.input(
                    InputKind::RoutePath,
                    "Chemin de la route (ex. /users/{id}/activate)",
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
                self.cli_job(args, "Création de route", None, false)?;
            }
            ChoiceKind::RouteConfirm => self.notice = "Création de route annulée".into(),
            _ => unreachable!(),
        }
        Ok(())
    }

    fn validate_route_draft(&self) -> Result<()> {
        let mut project = self.project.clone().context("Ouvre un projet d’abord")?;
        project.routes.push(self.route_draft.clone());
        project.validate()
    }

    fn confirm_route(&mut self) {
        let route = &self.route_draft;
        self.choose(
            ChoiceKind::RouteConfirm,
            "Créer cette route ?",
            &["Créer les fichiers", "Annuler"],
            vec![
                format!("{} {}", route.method, route.path),
                format!(
                    "Modèle : {} · corps : {}",
                    route.resource.as_deref().unwrap_or("indépendante"),
                    if route.json_body { "JSON" } else { "aucun" }
                ),
                format!("Dossier : src/resources/custom_routes/{}/", route.name),
                "handlers.rs : HTTP · requests.rs : paramètres · service.rs : métier".into(),
                "Réponse initiale : 501 non implémenté. Complète service.rs dans ton éditeur."
                    .into(),
                "Aucune migration SQL. Redémarre l’API pour charger la route.".into(),
            ],
            0,
        );
    }

    pub(super) fn preview_route(&mut self) -> Result<()> {
        let request = self
            .routes
            .get(self.route_selected)
            .context("Sélectionne une route")?;
        let project = self.project.as_ref().context("Ouvre un projet d’abord")?;
        let path = if let Some(route) = project
            .routes
            .iter()
            .find(|r| r.method == request.method && r.path == request.path)
        {
            format!("src/resources/custom_routes/{}/handlers.rs", route.name)
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
}
