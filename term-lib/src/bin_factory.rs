#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use derivative::*;
use serde::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use super::common::*;
use super::err;
use super::fs::TmpFileSystem;
use crate::api::*;
use crate::fd::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct BinaryPackage {
    #[derivative(Debug = "ignore")]
    pub data: Bytes,
    pub hash: String,
    pub chroot: bool,
    pub fs: TmpFileSystem,
    pub mappings: Vec<String>,
}

impl BinaryPackage {
    pub fn new(data: Bytes) -> BinaryPackage {
        let hash = hash_of_binary(&data);
        BinaryPackage {
            data,
            hash,
            chroot: false,
            fs: TmpFileSystem::default(),
            mappings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasConfig {
    pub run: String,
    #[serde(default)]
    pub chroot: bool,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub mappings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BinFactory {
    pub alias: Arc<RwLock<HashMap<String, Option<AliasConfig>>>>,
    pub cache: Arc<RwLock<HashMap<String, Option<BinaryPackage>>>>,
}

impl BinFactory {
    pub fn new() -> BinFactory {
        BinFactory {
            alias: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, name: &str, mut stderr: Fd) -> Option<BinaryPackage> {
        let mut name = name.to_string();

        // Fast path
        {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(&name) {
                return data.clone();
            }
        }

        // Tell the console we are fetching
        if stderr.is_tty() {
            stderr.write_clear_line().await;
            let _ = stderr.write("Fetching...".as_bytes()).await;
        } else {
            let _ = stderr
                .write(format!("[console] fetching '{}' from site", name).as_bytes())
                .await;
        }

        // Slow path
        let mut cache = self.cache.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return data.clone();
        }

        // First just try to find it
        if let Ok(data) = fetch_file(format!("/bin/{}.wasm", name).as_str())
            .join()
            .await
            .unwrap()
        {
            let data = BinaryPackage::new(Bytes::from(data));
            cache.insert(name, Some(data.clone()));
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return Some(data);
        }

        // NAK
        cache.insert(name, None);
        if stderr.is_tty() {
            stderr.write_clear_line().await;
        }
        return None;
    }

    pub async fn alias(&self, name: &str, mut stderr: Fd) -> Option<AliasConfig> {
        let mut name = name.to_string();

        // Fast path
        {
            let cache = self.alias.read().await;
            if let Some(data) = cache.get(&name) {
                return data.clone();
            }
        }

        // Tell the console we are fetching
        if stderr.is_tty() {
            stderr.write_clear_line().await;
            let _ = stderr.write("Probing...".as_bytes()).await;
        } else {
            let _ = stderr
                .write(format!("[console] probing for alias of '{}'", name).as_bytes())
                .await;
        }

        // Slow path
        let mut cache = self.alias.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return data.clone();
        }

        // Try and find it via a fetch
        let alias_path = format!("/bin/{}.alias", name);
        if let Ok(data) = fetch_file(alias_path.as_str()).join().await.unwrap() {
            // Decode the file into a yaml configuration
            match serde_yaml::from_slice::<AliasConfig>(&data[..]) {
                Ok(alias) => {
                    info!("binary alias '{}' found for {}", alias.run, name);
                    cache.insert(name, Some(alias.clone()));
                    if stderr.is_tty() {
                        stderr.write_clear_line().await;
                    }
                    return Some(alias);
                }
                Err(err) => {
                    warn!("alias file corrupt: {}", alias_path);
                }
            }
        }

        // NAK
        cache.insert(name, None);
        if stderr.is_tty() {
            stderr.write_clear_line().await;
        }
        return None;
    }
}

fn fetch_file(path: &str) -> AsyncResult<Result<Vec<u8>, i32>> {
    let system = System::default();
    system.fetch_file(path)
}

fn hash_of_binary(data: &Bytes) -> String {
    let mut hasher = Sha256::default();
    hasher.update(data.as_ref());
    let hash = hasher.finalize();
    base64::encode(&hash[..])
}
