use colored::*;
use std::path::PathBuf;
pub fn request_cretate_folder(project_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Create your api folder here ? y/n ".green());
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() != "y" {
        return Err("Aborted".into());
    }
    std::fs::create_dir_all(project_path)?;
    Ok(())
}

pub fn map_type(t: &str) -> &str {
    match t {
        "String" | "string" => "String",
        "Int" | "int" => "i32",
        "Binary" => "u8",
        _ => "String",
    }
}

pub fn map_type_sql(rust_type: &str) -> &str {
    match rust_type {
        "String" => "VARCHAR(255)",
        "i32" => "INTEGER",
        "i64" => "BIGINT",
        "bool" => "BOOLEAN",
        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",
        "u8" => "BINARY(16)",
        _ => "TEXT", // fallback
    }
}
