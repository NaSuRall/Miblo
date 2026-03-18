use serde_json::Value;

pub fn generate_model(models: &Vec<Value>) -> Vec<(String, String)> {
   // println!("GENERATED MODEL, {:#?}", model);
   let mut results = Vec::new();
   // Pour chaque model recus
   for model in models {
      // Prendre le nom les champs et ecrire la struct
      let name = model["name"].as_str().expect("Impossible de recuprere les nom du model");
      let fields = model["field"].as_object().expect("field doit être un objet");
      let mut result = format!("pub struct {} {{\n", name);

      // Pour chaque champs, on les met au bon Type, ex : string devient String Int devient i32...
      for (field_name, field_type) in fields {
         let field_type_str = field_type.as_str().expect("field type");
         let rust_type = map_type(field_type_str);
         result.push_str(&format!("    pub {}: {},\n", field_name, rust_type));
      }
      // On met le resultat dans result ce qui nous ecris la struct en str
      result.push_str("}\n\n");
      results.push((name.to_string(), result));

   }
   results
}
fn map_type(t: &str) -> &str {
   match t {
      "String" | "string" => "String",
      "Int" | "int" => "i32",
      _ => "String",
   }
}