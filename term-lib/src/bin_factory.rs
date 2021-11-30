#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use derivative::*;
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
    pub fs: TmpFileSystem,
}

impl BinaryPackage {
    pub fn new(data: Bytes) -> BinaryPackage {
        let hash = hash_of_binary(&data);
        BinaryPackage {
            data,
            hash,
            fs: TmpFileSystem::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinFactory {
    pub alias: Arc<RwLock<HashMap<String, Option<String>>>>,
    pub cache: Arc<RwLock<HashMap<String, Option<BinaryPackage>>>>,
}

impl BinFactory {
    pub fn new() -> BinFactory {
        BinFactory {
            alias: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, name: &str, mut stdout: Fd) -> Option<BinaryPackage> {
        let mut name = name.to_string();

        // Fast path
        {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(&name) {
                return data.clone();
            }
        }

        // Tell the console we are fetching
        stdout.write_clear_line().await;
        let _ = stdout.write("Fetching...".as_bytes()).await;

        // Slow path
        let mut cache = self.cache.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
            stdout.write_clear_line().await;
            return data.clone();
        }

        // First just try to find it
        if let Ok(data) = fetch_file(format!("/bin/{}.wasm", name).as_str()).await {
            let data = BinaryPackage::new(Bytes::from(data));
            cache.insert(name, Some(data.clone()));
            stdout.write_clear_line().await;
            return Some(data);
        }

        // NAK
        cache.insert(name, None);
        stdout.write_clear_line().await;
        return None;
    }

    pub async fn alias(&self, name: &str, mut stdout: Fd) -> Option<String> {
        let mut name = name.to_string();

        // Fast path
        {
            let cache = self.alias.read().await;
            if let Some(data) = cache.get(&name) {
                return data.clone();
            }
        }

        // Tell the console we are fetching
        stdout.write_clear_line().await;
        let _ = stdout.write("Probing...".as_bytes()).await;

        // Slow path
        let mut cache = self.alias.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
            stdout.write_clear_line().await;
            return data.clone();
        }

        // Try and find it via a fetch
        if let Ok(data) = fetch_file(format!("/bin/{}.alias", name).as_str()).await {
            let alias = String::from_utf8_lossy(&data[..]).trim().to_string();
            info!("binary alias '{}' found for {}", alias, name);
            cache.insert(name, Some(alias.clone()));
            stdout.write_clear_line().await;
            return Some(alias);
        }

        // NAK
        cache.insert(name, None);
        stdout.write_clear_line().await;
        return None;
    }
}

async fn fetch_file(path: &str) -> Result<Vec<u8>, i32> {
    let system = System::default();
    system.fetch_file(path).await
}

fn hash_of_binary(data: &Bytes) -> String {
    let mut hasher = Sha256::default();
    hasher.update(data.as_ref());
    let hash = hasher.finalize();
    base64::encode(&hash[..])
}
