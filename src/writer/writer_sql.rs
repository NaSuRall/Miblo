use std::collections::HashMap;
use std::io::Write;
use std::env::{self};
use std::fs::{File,create_dir_all};
use std::path::PathBuf;


pub fn writer(
    project_path: &PathBuf, 
    get: Vec<(String, String)>,
    post: Vec<(String, String)>,
    patch: Vec<(String, String)>,
    delete: Vec<(String, String)>,) 
    -> Result<(), Box<dyn std::error::Error>>{


    // let current_dir = env::current_dir()?;
    //let project_path = current_dir.join(name);
    let sql_dir = project_path.join("src/sql");


    let groups = HashMap::from([
        ("get", get),
        ("post", post),
        ("patch", patch),
        ("delete", delete)
    ]);
    
   
    for( method, content) in groups {
        
        for (model_name , content_method ) in content {
         
            let dir_name = sql_dir.join(model_name.to_lowercase());
            create_dir_all(&dir_name)?; 
            let file_name = format!("{}.sql", method);
            let file_path = dir_name.join(&file_name);
            let mut file = File::create(file_path)?;
            let _ = file.write_all(content_method.as_bytes());
        }
        
    }

    Ok(())
}
