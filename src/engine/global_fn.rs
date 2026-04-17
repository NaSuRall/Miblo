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
