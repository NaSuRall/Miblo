//! Raw YAML file reader.

use std::path::PathBuf;
use serde_yaml::Value;

/// Open and deserialize a YAML file at `template_dir` into a [`serde_yaml::Value`].
///
/// # Errors
///
/// Returns an error if the file cannot be opened or if the content is not valid YAML.
pub fn reader(tempalte_dir: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let reader_value = std::fs::File::open(tempalte_dir).expect("Failed to open config");
    let data: serde_yaml::Value = serde_yaml::from_reader(reader_value).expect("failed to read reader_value");
    Ok(data)
}
