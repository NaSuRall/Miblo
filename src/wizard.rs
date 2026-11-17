use crate::config::{FieldType, Resource, parse_field};
use anyhow::{Result, ensure};
use dialoguer::{Input, MultiSelect, Select, theme::ColorfulTheme};
use std::io::{self, IsTerminal};

#[derive(Debug)]
pub struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Opération annulée")
    }
}
impl std::error::Error for Cancelled {}

pub fn interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
}

fn require_terminal() -> Result<()> {
    ensure!(
        interactive(),
        "Interactive input requires a terminal; supply arguments and --yes instead"
    );
    Ok(())
}

pub fn select(label: &str, items: &[&str], default: usize) -> Result<Option<usize>> {
    require_terminal()?;
    Ok(Select::with_theme(&ColorfulTheme::default())
        .with_prompt(label)
        .items(items)
        .default(default)
        .interact_opt()?)
}

pub fn prompt_validated(
    label: &str,
    default: &str,
    validate: impl Fn(&str) -> Result<()>,
) -> Result<String> {
    require_terminal()?;
    let theme = ColorfulTheme::default();
    let mut input = Input::<String>::with_theme(&theme)
        .with_prompt(label)
        .validate_with(|value: &String| validate(value.trim()).map_err(|e| e.to_string()));
    if !default.is_empty() {
        input = input.default(default.into());
    }
    Ok(input.interact_text()?.trim().to_owned())
}

pub fn yes_no(label: &str, default: bool) -> Result<bool> {
    match select(label, &["Oui", "Non"], usize::from(!default))? {
        Some(index) => Ok(index == 0),
        None => Err(Cancelled.into()),
    }
}

pub fn confirm(label: &str) -> Result<()> {
    if yes_no(label, true)? {
        Ok(())
    } else {
        Err(Cancelled.into())
    }
}

pub fn resource(name: Option<String>) -> Result<Resource> {
    let name = match name {
        Some(name) => name,
        None => prompt_validated("Nom de la ressource (ex. Product)", "Product", |name| {
            Resource::new(name.into(), Vec::new()).validate_identity()
        })?,
    };
    Resource::new(name.clone(), Vec::new()).validate_identity()?;
    println!("Le champ id est créé automatiquement.");
    let mut fields = Vec::new();
    loop {
        let field_name = prompt_validated("Nom du champ (ex. title)", "", |value| {
            let mut candidate = fields.clone();
            candidate.push(parse_field(&format!("{value}:string"))?);
            Resource::new(name.clone(), candidate).validate()
        })?;
        let types = [
            ("Texte court — jusqu’à 255 caractères", "string"),
            ("Texte long", "text"),
            ("Nombre entier", "int"),
            ("Nombre décimal", "float"),
            ("Oui / Non", "bool"),
        ];
        let labels: Vec<_> = types.iter().map(|(label, _)| *label).collect();
        let kind = select("Type du champ", &labels, 0)?.ok_or(Cancelled)?;
        let mut field = parse_field(&format!("{field_name}:{}", types[kind].1))?;
        let force_private = field.private;
        let mut options = vec![("Facultatif — autoriser une valeur nulle", "nullable")];
        if field.kind != FieldType::Text {
            options.push(("Unique — interdire les doublons", "unique"));
        }
        if !force_private {
            options.push(("Privé — masquer dans les réponses de l’API", "private"));
        }
        if force_private {
            println!("Ce champ sensible sera automatiquement masqué dans les réponses.");
        }
        let labels: Vec<_> = options.iter().map(|(label, _)| *label).collect();
        println!("↑ ↓ : déplacer • Espace : cocher • Entrée : valider • Échap : annuler");
        let selections = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Options du champ")
            .items(&labels)
            .interact_opt()?
            .ok_or(Cancelled)?;
        for index in selections {
            match options[index].1 {
                "nullable" => field.nullable = true,
                "unique" => field.unique = true,
                "private" => field.private = true,
                _ => unreachable!("options are defined above"),
            }
        }
        let mut candidate = fields.clone();
        candidate.push(field);
        Resource::new(name.clone(), candidate.clone()).validate()?;
        fields = candidate;
        println!("Champ ajouté : {field_name} ({})", types[kind].0);
        if !yes_no("Ajouter un autre champ ?", true)? {
            break;
        }
    }
    let mut resource = Resource::new(name, fields);
    resource.path = prompt_validated("Adresse de la collection", &resource.path, |path| {
        let mut candidate = resource.clone();
        candidate.path = path.into();
        candidate.validate()
    })?;
    Ok(resource)
}
