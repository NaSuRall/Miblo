use crate::{
    config::{
        CustomRoute, Database, HTTP_METHODS, Project, Resource, RouteAction, parse_field,
        project_name,
    },
    generation, runtime,
    templates::{self, Pack},
    wizard,
};
use anyhow::{Context, Result, ensure};
use clap::{Args, CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "miblo",
    version,
    about = "Build and manage Rust / Axum APIs with SQLite or MySQL"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Create an API using the guided wizard or explicit arguments.
    New(NewArgs),
    /// Open the full-screen API workspace (also the default in a terminal).
    Ui {
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// Add a resource without rewriting existing resource files.
    Add(AddArgs),
    /// Add optional fields or relations to an existing model, preserving data and custom code.
    Edit(EditArgs),
    /// Create a model CRUD route or a custom business function.
    Route {
        #[command(subcommand)]
        command: RouteCommand,
    },
    /// Run the API; compilation and runtime errors remain visible.
    Run {
        #[arg(long, alias = "name", default_value = ".")]
        project: PathBuf,
    },
    /// Check project metadata, route registry and local prerequisites.
    Doctor {
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Check the database or apply pending project migrations.
    Db {
        #[arg(value_enum)]
        action: crate::database::Action,
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Update templates for future models without rewriting application code.
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
}

#[derive(Subcommand)]
enum TemplateCommand {
    Update(TemplateArgs),
}

#[derive(Subcommand)]
enum RouteCommand {
    Add(RouteArgs),
}

#[derive(Args)]
struct RouteArgs {
    /// Snake_case name for the generated module and function.
    name: String,
    #[arg(long, value_parser = HTTP_METHODS.to_vec(), ignore_case = true)]
    method: Option<String>,
    /// Path such as /users/active or /users/{id}/activate.
    #[arg(long)]
    path: String,
    /// Optional existing model associated with the route.
    #[arg(long)]
    resource: Option<String>,
    /// Reuse the model's SQL CRUD. Method and JSON contract are inferred.
    #[arg(long, value_enum, requires = "resource")]
    action: Option<RouteAction>,
    /// Extract a JSON request body (POST, PUT, PATCH or DELETE).
    #[arg(long)]
    json_body: bool,
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(short, long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
struct TemplateArgs {
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(long, conflicts_with = "template_url")]
    template_dir: Option<PathBuf>,
    #[arg(long)]
    template_url: Option<String>,
    #[arg(long)]
    template_version: Option<String>,
    #[arg(long, env = "MIBLO_TEMPLATE_BASE_URL", default_value = templates::DEFAULT_BASE)]
    template_base_url: String,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
struct CacheArgs {
    /// Reuse a cached template without accessing the network.
    #[arg(long)]
    offline: bool,
    #[arg(long, env = "MIBLO_CACHE_DIR")]
    cache_dir: Option<PathBuf>,
}
impl CacheArgs {
    fn path(&self) -> PathBuf {
        self.cache_dir
            .clone()
            .unwrap_or_else(templates::default_cache)
    }
}

#[derive(Args)]
struct NewArgs {
    name: Option<String>,
    /// Output directory (defaults to the project name).
    #[arg(long)]
    output: Option<PathBuf>,
    /// Read a version 1 project definition, instead of asking questions.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Use an unpacked template pack during local development.
    #[arg(long, env = "MIBLO_TEMPLATE_DIR", conflicts_with_all = ["template_url", "sha256"])]
    template_dir: Option<PathBuf>,
    #[arg(long)]
    template_version: Option<String>,
    /// Database backend (interactive default: SQLite; script default: MySQL).
    #[arg(long, value_enum)]
    database: Option<Database>,
    /// Base URL containing v<VERSION>/<STACK>.tar.gz.
    #[arg(long, env = "MIBLO_TEMPLATE_BASE_URL", default_value = templates::DEFAULT_BASE)]
    template_base_url: String,
    /// Direct archive URL; its checksum is fetched from <URL>.sha256.
    #[arg(long)]
    template_url: Option<String>,
    /// Pin an archive digest without fetching its checksum sidecar.
    #[arg(long, conflicts_with = "template_dir")]
    sha256: Option<String>,
    #[arg(long, default_value_t = 3000)]
    port: u16,
    #[arg(long)]
    resource: Option<String>,
    #[arg(long = "field", requires = "resource")]
    fields: Vec<String>,
    /// Accept supplied choices and defaults without interactive input.
    #[arg(short, long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
#[command(
    args_conflicts_with_subcommands = true,
    subcommand_precedence_over_arg = true
)]
struct AddArgs {
    #[command(flatten)]
    resource: ResourceArgs,
    #[command(subcommand)]
    legacy: Option<AddCommand>,
}

#[derive(Args)]
struct EditArgs {
    name: String,
    /// New optional fields, for example nickname:string? group_id:int?:ref=Group.
    #[arg(value_name = "FIELD", conflicts_with = "fields")]
    positional_fields: Vec<String>,
    #[arg(long = "field", value_name = "FIELD")]
    fields: Vec<String>,
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(short, long)]
    yes: bool,
    /// Preview SQL and affected files without changing the project or database.
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Subcommand)]
enum AddCommand {
    /// Compatibility with miblo add resource <NAME>.
    #[command(hide = true)]
    Resource(ResourceArgs),
}

#[derive(Args)]
struct ResourceArgs {
    /// Resource name, for example Product.
    name: Option<String>,
    /// Optional compact fields, for example title:string price:float.
    #[arg(value_name = "FIELD", conflicts_with = "fields")]
    positional_fields: Vec<String>,
    #[arg(long = "field", value_name = "FIELD")]
    fields: Vec<String>,
    #[arg(long)]
    path: Option<String>,
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(short, long)]
    yes: bool,
    #[arg(long)]
    dry_run: bool,
    #[command(flatten)]
    cache: CacheArgs,
}

pub fn run() -> Result<()> {
    match Cli::parse().command {
        Some(command) => execute(command),
        None if wizard::interactive() => crate::tui::run(None),
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}

fn execute(command: Command) -> Result<()> {
    match command {
        Command::New(args) => new(args),
        Command::Ui { project } => crate::tui::run(project),
        Command::Route {
            command: RouteCommand::Add(args),
        } => {
            let route = CustomRoute {
                name: args.name,
                method: args
                    .method
                    .map(|m| m.to_uppercase())
                    .unwrap_or_else(|| args.action.map_or("GET", RouteAction::method).into()),
                path: args.path,
                resource: args.resource,
                json_body: args.json_body || args.action.is_some_and(RouteAction::json_body),
                action: args.action,
            };
            if !args.yes && !args.dry_run {
                generation::add_route(
                    &args.project,
                    route.clone(),
                    args.cache.offline,
                    &args.cache.path(),
                    true,
                )?;
                wizard::confirm(if route.action.is_some() {
                    "Créer cette route reliée au CRUD du modèle ? Aucune nouvelle table."
                } else {
                    "Créer cette route à compléter dans l’éditeur ? Aucune migration SQL."
                })?;
            }
            generation::add_route(
                &args.project,
                route,
                args.cache.offline,
                &args.cache.path(),
                args.dry_run,
            )
        }
        Command::Edit(args) => {
            let fields = if args.positional_fields.is_empty() {
                args.fields
            } else {
                args.positional_fields
            };
            let fields = fields
                .iter()
                .map(|f| parse_field(f))
                .collect::<Result<Vec<_>>>()?;
            // Build a reviewable plan before asking for confirmation.
            if !args.yes && !args.dry_run {
                generation::edit(
                    &args.project,
                    &args.name,
                    fields.clone(),
                    args.cache.offline,
                    &args.cache.path(),
                    true,
                )?;
                wizard::confirm(&format!(
                    "Ajouter ces champs à {} ? La base sera migrée séparément.",
                    args.name
                ))?;
            }
            generation::edit(
                &args.project,
                &args.name,
                fields,
                args.cache.offline,
                &args.cache.path(),
                args.dry_run,
            )
        }
        Command::Add(args) => add(match args.legacy {
            Some(AddCommand::Resource(resource)) => resource,
            None => args.resource,
        }),
        Command::Run { project } => runtime::start(&project),
        Command::Doctor { project } => runtime::doctor(&project),
        Command::Db { action, project } => crate::database::run(&project, action),
        Command::Template {
            command: TemplateCommand::Update(args),
        } => {
            let (project, _) = generation::inspect(&args.project)?;
            let pack = if let Some(path) = args.template_dir {
                Pack::local(&path)?
            } else {
                let version = args
                    .template_version
                    .unwrap_or_else(|| project.database().version().into());
                ensure!(
                    !version.is_empty()
                        && version
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c)),
                    "Invalid template version"
                );
                let url = args.template_url.unwrap_or_else(|| {
                    format!(
                        "{}/v{}/{}.tar.gz",
                        args.template_base_url.trim_end_matches('/'),
                        version,
                        project.stack
                    )
                });
                Pack::remote(&url, &version, None, args.cache.offline, &args.cache.path())?
            };
            generation::update_template(&args.project, &pack, args.dry_run)
        }
    }
}

fn add(args: ResourceArgs) -> Result<()> {
    let fields = if args.positional_fields.is_empty() {
        args.fields
    } else {
        args.positional_fields
    };
    let mut resource = if fields.is_empty() {
        ensure!(!args.yes, "Supply at least one field when using --yes");
        generation::inspect(&args.project)?;
        wizard::resource(args.name)?
    } else {
        Resource::new(
            args.name.context("Supply a resource name")?,
            fields
                .iter()
                .map(|f| parse_field(f))
                .collect::<Result<_>>()?,
        )
    };
    if let Some(path) = args.path {
        resource.path = path;
    }
    resource.validate()?;
    if !args.yes && !args.dry_run {
        wizard::confirm(&format!(
            "Ajouter {} dans {} ?",
            resource.name,
            args.project.display()
        ))?;
    }
    generation::add(
        &args.project,
        resource,
        args.cache.offline,
        &args.cache.path(),
        args.dry_run,
    )
}

fn new(args: NewArgs) -> Result<()> {
    let project = if let Some(path) = &args.config {
        ensure!(
            args.resource.is_none() && args.fields.is_empty(),
            "Do not combine --config with --resource/--field"
        );
        let project = Project::load(path)?;
        ensure!(
            args.database.is_none_or(|db| db == project.database()),
            "--database must match --config"
        );
        if let Some(name) = &args.name {
            ensure!(name == &project.name, "The name must match --config");
        }
        project
    } else {
        let name = match args.name {
            Some(name) => name,
            None => {
                ensure!(!args.yes, "Supply a project name with --yes");
                wizard::prompt_validated("Nom du projet", "my-api", project_name)?
            }
        };
        project_name(&name)?;
        let database = match args.database {
            Some(database) => database,
            None if args.yes => Database::Mysql,
            None => match wizard::select(
                "Base de données",
                &[
                    "SQLite — configuration automatique",
                    "MySQL — serveur existant",
                ],
                0,
            )? {
                Some(0) => Database::Sqlite,
                Some(_) => Database::Mysql,
                None => return Err(wizard::Cancelled.into()),
            },
        };
        let mut port = args.port;
        let mut resources = Vec::new();
        if let Some(name) = args.resource {
            if args.fields.is_empty() {
                ensure!(
                    !args.yes,
                    "Supply at least one --field with --resource and --yes"
                );
                resources.push(wizard::resource(Some(name))?);
            } else {
                resources.push(Resource::new(
                    name,
                    args.fields
                        .iter()
                        .map(|f| parse_field(f))
                        .collect::<Result<_>>()?,
                ));
            }
        } else if !args.yes {
            println!("API Rust • Axum • {}", database.name());
            port = wizard::prompt_validated("Port du serveur", &port.to_string(), |value| {
                ensure!(
                    value.parse::<u16>().is_ok_and(|port| port > 0),
                    "Choisis un port entre 1 et 65535"
                );
                Ok(())
            })?
            .parse()
            .context("Invalid port")?;
            loop {
                if !wizard::yes_no("Ajouter une ressource maintenant ?", true)? {
                    break;
                }
                resources.push(wizard::resource(None)?);
            }
        }
        let mut project = Project::new(name, port, resources);
        project.stack = database.stack().into();
        project
    };
    project.validate()?;
    let destination = args.output.unwrap_or_else(|| PathBuf::from(&project.name));
    ensure!(
        std::fs::symlink_metadata(&destination)
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "Destination exists or is inaccessible: {}",
        destination.display()
    );
    if !args.yes && !args.dry_run {
        wizard::confirm(&format!(
            "Créer {} avec {} ressource(s) ?",
            project.name,
            project.resources.len()
        ))?;
    }
    let pack = if let Some(path) = args.template_dir {
        Pack::local(&path)?
    } else {
        let version = args
            .template_version
            .unwrap_or_else(|| project.database().version().into());
        ensure!(
            !version.is_empty()
                && version
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-_".contains(&c)),
            "Invalid template version"
        );
        let url = args.template_url.unwrap_or_else(|| {
            format!(
                "{}/v{}/{}.tar.gz",
                args.template_base_url.trim_end_matches('/'),
                version,
                project.stack
            )
        });
        Pack::remote(
            &url,
            &version,
            args.sha256.as_deref(),
            args.cache.offline,
            &args.cache.path(),
        )?
    };
    generation::create(&project, &pack, &destination, args.dry_run)
}
