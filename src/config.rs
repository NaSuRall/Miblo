use anyhow::{Context, Result, bail, ensure};
use heck::{ToSnakeCase, ToUpperCamelCase};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, clap::ValueEnum)]
pub enum Database {
    #[default]
    Sqlite,
    Mysql,
}
impl Database {
    pub fn stack(self) -> &'static str {
        match self {
            Self::Sqlite => "rust-axum-sqlite",
            Self::Mysql => "rust-axum-mysql",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Sqlite => "SQLite",
            Self::Mysql => "MySQL",
        }
    }
    pub fn arg(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Mysql => "mysql",
        }
    }
    pub fn version(self) -> &'static str {
        match self {
            Self::Sqlite | Self::Mysql => "0.3.0",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub stack: String,
    pub port: u16,
    pub resources: Vec<Resource>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub name: String,
    pub path: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub private: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<Reference>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// PascalCase resource name; references always target its generated id.
    pub resource: String,
    #[serde(default)]
    pub on_delete: OnDelete,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnDelete {
    #[default]
    Restrict,
    Cascade,
    SetNull,
}
impl OnDelete {
    pub fn sql(self) -> &'static str {
        match self {
            Self::Restrict => "RESTRICT",
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
        }
    }
    pub fn arg(self) -> &'static str {
        match self {
            Self::Restrict => "restrict",
            Self::Cascade => "cascade",
            Self::SetNull => "set_null",
        }
    }
}

impl Field {
    pub fn argument(&self) -> String {
        format!(
            "{}:{}{}{}{}{}",
            self.name,
            match self.kind {
                FieldType::String => "string",
                FieldType::Text => "text",
                FieldType::Int => "int",
                FieldType::Bool => "bool",
                FieldType::Float => "float",
            },
            if self.nullable { "?" } else { "" },
            if self.unique { ":unique" } else { "" },
            if self.private { ":private" } else { "" },
            self.references.as_ref().map_or(String::new(), |r| format!(
                ":ref={}:on_delete={}",
                r.resource,
                r.on_delete.arg()
            ))
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Text,
    Int,
    Bool,
    Float,
}

impl FieldType {
    pub fn rust(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Int => "i64",
            Self::Bool => "bool",
            Self::Float => "f64",
        }
    }
    pub fn sql(self, database: Database) -> &'static str {
        if database == Database::Sqlite {
            return match self {
                Self::String | Self::Text => "TEXT",
                Self::Int | Self::Bool => "INTEGER",
                Self::Float => "REAL",
            };
        }
        match self {
            Self::String => "VARCHAR(255)",
            Self::Text => "TEXT",
            Self::Int => "BIGINT",
            Self::Bool => "BOOLEAN",
            Self::Float => "DOUBLE",
        }
    }
}

impl Project {
    pub fn new(name: String, port: u16, resources: Vec<Resource>) -> Self {
        Self {
            schema_version: 1,
            name,
            stack: "rust-axum-mysql".into(),
            port,
            resources,
        }
    }
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))?;
        let project: Self = serde_yaml::from_str(&text).context(
            "Invalid miblo.yaml (legacy config.yaml files need to be migrated; see README)",
        )?;
        project.validate()?;
        Ok(project)
    }
    pub fn database(&self) -> Database {
        if self.stack == "rust-axum-sqlite" {
            Database::Sqlite
        } else {
            Database::Mysql
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unsupported project schema version"
        );
        project_name(&self.name)?;
        ensure!(
            matches!(self.stack.as_str(), "rust-axum-mysql" | "rust-axum-sqlite"),
            "Supported stacks: rust-axum-mysql and rust-axum-sqlite"
        );
        ensure!(self.port > 0, "The server port must be greater than zero");
        let mut names = HashSet::new();
        let mut paths = HashSet::new();
        let mut tables = HashSet::new();
        for resource in &self.resources {
            resource.validate()?;
            ensure!(
                names.insert(resource.module()),
                "Duplicate resource: {}",
                resource.name
            );
            ensure!(
                paths.insert(&resource.path),
                "Duplicate resource path: {}",
                resource.path
            );
            ensure!(
                tables.insert(resource.table()),
                "Duplicate database table: {}",
                resource.table()
            );
        }
        // Edits can reference tables created after this resource's first migration.
        for resource in &self.resources {
            for field in &resource.fields {
                if let Some(reference) = &field.references {
                    ensure!(
                        self.resources.iter().any(|r| r.name == reference.resource),
                        "{}.{} references {}: create that model first",
                        resource.name,
                        field.name,
                        reference.resource
                    );
                }
            }
        }
        Ok(())
    }
}

pub fn project_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name.len() <= 48
            && name.as_bytes()[0].is_ascii_lowercase()
            && name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' || c == b'-'),
        "Project name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (max 48)"
    );
    identifier(&name.replace('-', "_"))?;
    ensure!(
        ![
            "test",
            "core",
            "std",
            "alloc",
            "axum",
            "sqlx",
            "tokio",
            "serde",
            "serde_json",
            "dotenvy",
            "tower"
        ]
        .contains(&name.replace('-', "_").as_str()),
        "Reserved project name: {name}"
    );
    Ok(())
}

fn identifier(name: &str) -> Result<()> {
    const RESERVED: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while", "abstract", "become", "box", "do",
        "final", "gen", "macro", "override", "priv", "try", "typeof", "unsized", "virtual",
        "yield",
    ];
    ensure!(
        !name.is_empty()
            && name.len() <= 48
            && name.as_bytes()[0].is_ascii_alphabetic()
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
            && !RESERVED.contains(&name),
        "Invalid or reserved identifier: {name}"
    );
    ensure!(
        ![
            "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
            "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"
        ]
        .contains(&name.to_lowercase().as_str()),
        "Reserved file name: {name}"
    );
    Ok(())
}

impl Resource {
    pub fn new(name: String, fields: Vec<Field>) -> Self {
        let table = plural(&name.to_snake_case());
        Self {
            name,
            path: format!("/{table}"),
            fields,
        }
    }
    pub fn module(&self) -> String {
        self.name.to_snake_case()
    }
    pub fn table(&self) -> String {
        plural(&self.module())
    }
    pub fn validate_identity(&self) -> Result<()> {
        identifier(&self.name)?;
        ensure!(
            self.name == self.name.to_upper_camel_case(),
            "Resource names must use PascalCase, for example Product or OrderItem"
        );
        identifier(&self.module())?;
        ensure!(
            ![
                "Router",
                "AppState",
                "ApiError",
                "Create",
                "Update",
                "Pagination",
                "Option",
                "Result",
                "String",
                "Vec",
                "Json",
                "State",
                "Path",
                "Query",
                "StatusCode",
                "MySql",
                "Sqlite",
                "QueryBuilder"
            ]
            .contains(&self.name.as_str()),
            "Reserved resource name: {}",
            self.name
        );
        let path = self.path.strip_prefix('/').unwrap_or("");
        ensure!(
            !path.is_empty()
                && path.len() <= 64
                && path.bytes().all(|c| c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || c == b'-'
                    || c == b'_'),
            "Resource path must be a single lowercase segment, for example /products"
        );
        ensure!(path != "health", "The /health path is reserved");
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.validate_identity()?;
        ensure!(
            !self.fields.is_empty() && self.fields.len() <= 64,
            "A resource needs between 1 and 64 fields (id is generated automatically)"
        );
        let mut fields = HashSet::new();
        for field in &self.fields {
            identifier(&field.name)?;
            ensure!(
                field.name == field.name.to_snake_case() && field.name != "id",
                "Field names must be snake_case; id is generated automatically"
            );
            ensure!(
                fields.insert(&field.name),
                "Duplicate field: {}",
                field.name
            );
            ensure!(
                !(field.unique && field.kind == FieldType::Text),
                "Use string rather than text for unique field {}",
                field.name
            );
            ensure!(
                !sensitive(&field.name) || field.private,
                "Sensitive field {} must be marked private",
                field.name
            );
            if let Some(reference) = &field.references {
                Resource::new(reference.resource.clone(), vec![]).validate_identity()?;
                ensure!(
                    field.kind == FieldType::Int,
                    "Foreign key {} must use int",
                    field.name
                );
                ensure!(
                    reference.on_delete != OnDelete::SetNull || field.nullable,
                    "SET NULL requires a nullable field: {}",
                    field.name
                );
            }
        }
        Ok(())
    }
}

fn plural(name: &str) -> String {
    if name.ends_with('y') && name.len() > 1 && !b"aeiou".contains(&name.as_bytes()[name.len() - 2])
    {
        format!("{}ies", &name[..name.len() - 1])
    } else if name.ends_with('s') {
        name.into()
    } else {
        format!("{name}s")
    }
}

fn sensitive(name: &str) -> bool {
    ["password", "password_hash", "secret", "token", "api_key"].contains(&name)
}

pub fn parse_field(value: &str) -> Result<Field> {
    let parts: Vec<_> = value.split(':').collect();
    ensure!(
        parts.len() >= 2,
        "Use name:type, for example title:string or description:text?"
    );
    let nullable = parts[1].ends_with('?');
    let kind = match parts[1].trim_end_matches('?') {
        "string" => FieldType::String,
        "text" => FieldType::Text,
        "int" => FieldType::Int,
        "bool" => FieldType::Bool,
        "float" => FieldType::Float,
        other => bail!("Unknown type {other}; choose string, text, int, bool or float"),
    };
    let mut field = Field {
        name: parts[0].into(),
        kind,
        nullable,
        unique: false,
        private: sensitive(parts[0]),
        references: None,
    };
    let mut on_delete = None;
    for flag in &parts[2..] {
        match *flag {
            "unique" => field.unique = true,
            "private" => field.private = true,
            flag if flag.starts_with("ref=") => {
                ensure!(field.references.is_none(), "Duplicate foreign key option");
                field.references = Some(Reference {
                    resource: flag[4..].into(),
                    on_delete: OnDelete::Restrict,
                });
            }
            flag if flag.starts_with("on_delete=") => {
                ensure!(on_delete.is_none(), "Duplicate on_delete option");
                on_delete = Some(match &flag[10..] {
                    "restrict" => OnDelete::Restrict,
                    "cascade" => OnDelete::Cascade,
                    "set_null" => OnDelete::SetNull,
                    _ => bail!("Use on_delete=restrict, cascade or set_null"),
                });
            }
            _ => bail!(
                "Unknown field option {flag}; choose unique, private, ref=Model or on_delete=restrict|cascade|set_null"
            ),
        }
    }
    if let Some(on_delete) = on_delete {
        field
            .references
            .as_mut()
            .context("on_delete requires ref=Model")?
            .on_delete = on_delete;
    }
    Ok(field)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relations_validate_targets_types_and_actions() {
        let group = Resource::new("Group".into(), vec![parse_field("name:string").unwrap()]);
        let field = parse_field("group_id:int?:unique:ref=Group:on_delete=set_null").unwrap();
        assert_eq!(
            field.argument(),
            "group_id:int?:unique:ref=Group:on_delete=set_null"
        );
        let member = Resource::new("Member".into(), vec![field]);
        Project::new("example".into(), 3000, vec![group.clone(), member.clone()])
            .validate()
            .unwrap();
        assert!(
            Project::new("example".into(), 3000, vec![member.clone(), group.clone()])
                .validate()
                .is_ok()
        );
        for field in [
            "group_id:string:ref=Group",
            "group_id:int:ref=Group:on_delete=set_null",
            "group_id:int:ref=Group;DROP",
            "group_id:int:ref=Missing",
        ] {
            let resource = Resource::new("Member".into(), vec![parse_field(field).unwrap()]);
            assert!(
                Project::new("example".into(), 3000, vec![group.clone(), resource])
                    .validate()
                    .is_err(),
                "{field}"
            );
        }
        for field in [
            "group_id:int:on_delete=cascade",
            "group_id:int:ref=Group:ref=Group",
            "group_id:int:ref=Group:on_delete=drop",
        ] {
            assert!(parse_field(field).is_err());
        }
        let folder = Resource::new(
            "Folder".into(),
            vec![parse_field("parent_id:int?:ref=Folder:on_delete=set_null").unwrap()],
        );
        Project::new("example".into(), 3000, vec![folder])
            .validate()
            .unwrap();
    }
    #[test]
    fn absent_relations_preserve_legacy_serialization() {
        let original = "schema_version: 1\nname: sample\nstack: rust-axum-mysql\nport: 3000\nresources:\n- name: User\n  path: /users\n  fields:\n  - name: name\n    type: string\n    nullable: false\n    unique: false\n    private: false\n";
        let config: Project = serde_yaml::from_str(original).unwrap();
        assert_eq!(serde_yaml::to_string(&config).unwrap(), original);
    }
}
