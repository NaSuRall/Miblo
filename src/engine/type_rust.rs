//! Miblo type-name → Rust / SQL type-string mappers.

/// Map a Miblo type name to its Rust equivalent.
///
/// | Miblo type | Rust type |
/// |------------|-----------|
/// | `"String"` / `"string"` | `"String"` |
/// | `"Int"` / `"int"` | `"i32"` |
/// | `"Binary"` | `"u8"` |
/// | *(anything else)* | `"String"` *(fallback)* |
pub fn map_type(t: &str) -> &str {
    match t {
        "String" | "string" => "String",
        "Int" | "int" => "i32",
        "Binary" => "u8",
        _ => "String",
    }
}

/// Map a Rust type name to its SQL column type.
///
/// Used by the SQL and migration generators to emit correct DDL.
///
/// | Rust type | SQL type |
/// |-----------|----------|
/// | `"String"` | `"VARCHAR(255)"` |
/// | `"i32"` | `"INTEGER"` |
/// | `"i64"` | `"BIGINT"` |
/// | `"bool"` | `"BOOLEAN"` |
/// | `"f32"` | `"REAL"` |
/// | `"f64"` | `"DOUBLE PRECISION"` |
/// | `"u8"` | `"BINARY(16)"` |
/// | *(anything else)* | `"TEXT"` *(fallback)* |
pub fn map_type_sql(rust_type: &str) -> &str {
    match rust_type {
        "String" => "VARCHAR(255)",
        "i32" => "INTEGER",
        "i64" => "BIGINT",
        "bool" => "BOOLEAN",
        "f32" => "REAL",
        "f64" => "DOUBLE PRECISION",
        "u8" => "BINARY(16)",
        _ => "TEXT",
    }
}
