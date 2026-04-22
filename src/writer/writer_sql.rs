use std::io::Write;
use std::env::{self};
use std::fs::{File};



pub fn writer(
    name: &str, 
    get: Vec<(String, String)>,
    post: Vec<(String, String)>,
    patch: Vec<(String, String)>,
    delete: Vec<(String, String)>,) 
    -> Result<(), Box<dyn std::error::Error>>{


    let current_dir = env::current_dir()?;
    let project_path = current_dir.join(name);
    let sql_dir = project_path.join("src/sql");


    for( model_name, content) in get {
        
        // Cree un fichier avec le nom du model
        
        // crée le fichier dans le dossier 
        let file_name = format!("get_{}.sql", model_name.to_lowercase());
        let file_path = sql_dir.join(&file_name);

        let mut file = File::create(file_path)?;
        // ecrire les data dans le fichier 
        file.write_all(content.as_bytes())?;
    }
   
    
    for( model_name, content) in post {
        
        let file_name = format!("post_{}.sql", model_name.to_lowercase());
        let file_path = sql_dir.join(&file_name);

        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
    }

    
    for( model_name, content) in patch {
        let file_name = format!("patch_{}.sql", model_name.to_lowercase());
        let file_path = sql_dir.join(&file_name);

        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
    }
    
    for( model_name, content) in delete {
        let file_name = format!("delete_{}.sql", model_name.to_lowercase());
        let file_path = sql_dir.join(&file_name);

        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
    }
     
    

    Ok(())
}
