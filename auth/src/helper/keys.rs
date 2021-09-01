#[allow(unused_imports)]
use tracing::{info, warn, debug, error, trace, instrument, span, Level};
use serde::*;
use std::fs::File;

pub fn try_load_key<T>(key_path: String) -> Option<T>
where T: serde::de::DeserializeOwned
{
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("loading key: {}", path);
    let path = std::path::Path::new(&path);
    File::open(path)
        .ok()
        .map(|file| {
            bincode::deserialize_from(&file).unwrap()
        })
}

pub fn load_key<T>(key_path: String, postfix: &str) -> T
where T: serde::de::DeserializeOwned
{
    let key_path = format!("{}{}", key_path, postfix).to_string();
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("loading key: {}", path);
    let path = std::path::Path::new(&path);
    let file = File::open(path).unwrap();
    bincode::deserialize_from(&file).unwrap()
}

pub fn save_key<T>(key_path: String, key: T, postfix: &str)
where T: Serialize
{
    let key_path = format!("{}{}", key_path, postfix).to_string();
    let path = shellexpand::tilde(&key_path).to_string();
    debug!("saving key: {}", path);
    let path = std::path::Path::new(&path);
    let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
    let mut file = File::create(path).unwrap();
    
    print!("Generating secret key at {}...", key_path);
    bincode::serialize_into(&mut file, &key).unwrap();
    println!("Done");
}