use std::path::PathBuf;
use serde_yaml::Value;

pub fn reader(tempalte_dir: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let reader_value = std::fs::File::open(tempalte_dir).expect("Failed to open config");
    let data: serde_yaml::Value = serde_yaml::from_reader(reader_value).expect("failed to read reader_value");
    Ok(data)
}
