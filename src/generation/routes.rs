use super::*;
use crate::{config::CustomRoute, templates::ROUTE_FILES};
use heck::ToSnakeCase;

const ROUTES: &str = "src/resources/custom_routes/mod.rs";

pub(super) fn supported(pack: &Pack, project: &Project) -> Result<()> {
    ensure!(
        project.routes.is_empty()
            || (pack.format >= 3 && pack.features.iter().any(|f| f == "custom-routes")),
        trf!(
            "This template does not support route creation yet. Press u in Miblo or use miblo template update (version 0.5.0 or later), then retry. No files changed.",
            "Ce template ne permet pas encore de créer des routes. Appuie sur u dans Miblo ou utilise miblo template update (version 0.5.0 ou ultérieure), puis réessaie. Aucun fichier modifié."
        )
    );
    ensure!(
        !project.routes.iter().any(|r| r.action.is_some())
            || pack.features.iter().any(|f| f == "model-routes"),
        trf!(
            "This template does not support model CRUD routes yet. Press u or use miblo template update (pack 0.6.0 or later). No files changed.",
            "Ce template ne permet pas encore les routes CRUD de modèles. Appuie sur u ou utilise miblo template update (pack 0.6.0 ou ultérieur). Aucun fichier modifié."
        )
    );
    Ok(())
}

fn route_files(pack: &Pack, route: &CustomRoute, files: &mut Files) -> Result<()> {
    let data = json!({ "name": route.name, "method": route.method.to_lowercase(), "path": route.path,
        "parameters": route.parameters(), "json_body": route.json_body,
        "action_handler": route.action.map(|a| a.handler()),
        "resource_module": route.resource.as_ref().map(|r| r.to_snake_case()) });
    for (template, output) in ROUTE_FILES {
        // Automatic routes share the model's handlers, validation and SQL. Only
        // the router and its explicit handler alias need their own files.
        if route.action.is_some() && matches!(*output, "requests.rs" | "service.rs") {
            continue;
        }
        files.insert(
            format!("src/resources/custom_routes/{}/{output}", route.name),
            render_source(pack, template, &data)?,
        );
    }
    Ok(())
}

fn registry(pack: &Pack, project: &Project, files: &mut Files) -> Result<()> {
    files.insert(
        ROUTES.into(),
        render_source(pack, "route/registry.rs.hbs", &project_data(project))?,
    );
    Ok(())
}

pub(super) fn create_files(pack: &Pack, project: &Project, files: &mut Files) -> Result<()> {
    supported(pack, project)?;
    if project.routes.is_empty() {
        return Ok(());
    }
    for route in &project.routes {
        route_files(pack, route, files)?;
    }
    files.insert(
        "src/resources/custom_routes/error.rs".into(),
        render_source(pack, "route/error.rs.hbs", &project_data(project))?,
    );
    registry(pack, project, files)
}

pub(super) fn inspect(root: &Path, project: &Project, state: &State) -> Result<()> {
    if !project.routes.is_empty() {
        safe_path(root, ROUTES)?;
        let ancestor = state
            .generated
            .get(ROUTES)
            .context("Missing custom route registry baseline")?;
        let current = fs::read_to_string(root.join(ROUTES))?;
        ensure!(
            edit::format_source(&current)? == edit::format_source(ancestor)?,
            trf!(
                "The custom route registry was modified. Keep your changes in handlers or services and restore {ROUTES} before generating code.",
                "Le registre des routes personnalisées a été modifié. Conserve tes changements dans les handlers ou services et restaure {ROUTES} avant de générer du code."
            )
        );
    }
    Ok(())
}

pub fn add_route(
    root: &Path,
    route: CustomRoute,
    offline: bool,
    cache: &Path,
    dry_run: bool,
) -> Result<()> {
    route.validate()?;
    let _lock = if dry_run {
        None
    } else {
        Some(OperationLock::acquire(root)?)
    };
    let mut originals = BTreeMap::new();
    for path in [CONFIG, STATE, REGISTRY] {
        originals.insert(path.into(), fs::read(root.join(path))?);
    }
    let (mut project, mut state) = super::inspect(root)?;
    pin_resource_templates(&project, &mut state);
    let first = project.routes.is_empty();
    if !first {
        originals.insert(ROUTES.into(), fs::read(root.join(ROUTES))?);
    }
    project.routes.push(route.clone());
    project.validate()?;
    let pack = Pack::pinned(&state.template, offline, cache)?;
    supported(&pack, &project)?;
    ensure!(
        pack.id == project.stack,
        "Template stack does not match the project"
    );
    let mut files = Files::new();
    route_files(&pack, &route, &mut files)?;
    if first {
        files.insert(
            "src/resources/custom_routes/error.rs".into(),
            render_source(&pack, "route/error.rs.hbs", &project_data(&project))?,
        );
    }
    registry(&pack, &project, &mut files)?;
    metadata(
        &pack,
        &project,
        state.next_migration,
        state.generated,
        state.resource_templates,
        &mut files,
    )?;
    preflight(root, &files, &originals)?;
    if dry_run {
        show_plan(&files);
        return Ok(());
    }
    commit(root, &files, &originals)?;
    println!(
        "{}",
        trf!(
            "Route created: {} {}",
            "Route créée : {} {}",
            route.method,
            route.path
        )
    );
    if let Some(action) = route.action {
        println!(
            "{}",
            trf!(
                "{} · CRUD and SQL for model {} connected automatically.",
                "{} · CRUD et SQL du modèle {} reliés automatiquement.",
                action.label(),
                route.resource.as_deref().unwrap_or_default()
            )
        );
        println!(
            "{}",
            trf!(
                "Restart the API to load this route, then test it with x. No new table.",
                "Redémarre l’API pour charger cette route, puis teste-la avec x. Aucune nouvelle table."
            )
        );
    } else {
        println!(
            "{}",
            trf!(
                "Complete src/resources/custom_routes/{}/service.rs in your editor.\nThe handler returns HTTP 501 until implemented. Restart the API to load this route. No SQL migration.",
                "Complète src/resources/custom_routes/{}/service.rs dans ton éditeur.\nLe handler renvoie HTTP 501 jusqu’à son implémentation. Redémarre l’API pour charger cette route. Aucune migration SQL.",
                route.name
            )
        );
    }
    Ok(())
}
