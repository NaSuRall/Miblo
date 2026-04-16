use std::string::String;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

pub fn write_migration( migration_dir: PathBuf, data: &[String]) {
    

    let mut file = OpenOptions::new()
        .append(true)
        .open(&migration_dir)
        .expect("Impossible d'ouvrir le fichier de migration");
    
    for line in data {

        writeln!(file, "{}", line)
        .expect("Erreur lors de l'ecriture");
    }

}
