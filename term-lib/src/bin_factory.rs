#![allow(dead_code)]
#![allow(unused)]
#[cfg(feature = "cached_compiling")]
use crate::wasmer::Module;
use bytes::Bytes;
use derivative::*;
use serde::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
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

#[derive(Debug, Default)]
#[cfg(feature = "cached_compiling")]
pub struct CachedCompiledModules {
    pub modules: RwLock<HashMap<String, Option<Module>>>,
}

#[cfg(feature = "cached_compiling")]
impl CachedCompiledModules {
    pub async fn get_compiled_module(&self, data_hash: &String) -> Option<Module> {
        let cache = self.modules.read().await;
        cache.get(data_hash).map(|a| a.clone()).flatten()
    }

    pub async fn set_compiled_module(&self, data_hash: String, compiled_module: Module) {
        let mut cache = self.modules.write().await;
        cache.insert(data_hash, Some(compiled_module));
    }
}

#[derive(Debug, Clone)]
pub struct BinFactory {
    pub wax: Arc<Mutex<HashSet<String>>>,
    pub alias: Arc<RwLock<HashMap<String, Option<AliasConfig>>>>,
    pub cache: Arc<RwLock<HashMap<String, Option<BinaryPackage>>>>,
    #[cfg(feature = "cached_compiling")]
    pub compiled_modules: Arc<CachedCompiledModules>,
}

impl BinFactory {
    pub fn new(
        #[cfg(feature = "cached_compiling")] compiled_modules: Arc<CachedCompiledModules>,
    ) -> BinFactory {
        BinFactory {
            wax: Arc::new(Mutex::new(HashSet::new())),
            alias: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "cached_compiling")]
            compiled_modules,
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
        #[cfg(target_arch = "wasm32")]
        {
            if stderr.is_tty() {
                stderr.write_clear_line().await;
                let _ = stderr.write("Fetching...".as_bytes()).await;
            } else {
                let _ = stderr
                    .write(format!("[console] fetching '{}' from site", name).as_bytes())
                    .await;
            }
        }

        // Slow path
        let mut cache = self.cache.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
            #[cfg(target_arch = "wasm32")]
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
            #[cfg(target_arch = "wasm32")]
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return Some(data);
        }

        // NAK
        cache.insert(name, None);
        #[cfg(target_arch = "wasm32")]
        if stderr.is_tty() {
            stderr.write_clear_line().await;
        }
        return None;
    }

    #[cfg(feature = "cached_compiling")]
    pub async fn get_compiled_module(&self, data_hash: &String) -> Option<Module> {
        self.compiled_modules.get_compiled_module(data_hash).await
    }

    #[cfg(feature = "cached_compiling")]
    pub async fn set_compiled_module(&self, data_hash: String, compiled_module: Module) {
        self.compiled_modules
            .set_compiled_module(data_hash, compiled_module)
            .await
    }

    pub async fn alias(&self, name: &str) -> Option<AliasConfig> {
        let mut name = name.to_string();

        // Fast path
        {
            let cache = self.alias.read().await;
            if let Some(data) = cache.get(&name) {
                return data.clone();
            }
        }

        // Slow path
        let mut cache = self.alias.write().await;

        // Check the cache
        if let Some(data) = cache.get(&name) {
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
                    return Some(alias);
                }
                Err(err) => {
                    warn!("alias file corrupt: {}", alias_path);
                }
            }
        }

        // NAK
        cache.insert(name, None);
        return None;
    }
}

fn fetch_file(path: &str) -> AsyncResult<Result<Vec<u8>, u32>> {
    let system = System::default();
    system.fetch_file(path)
}

fn hash_of_binary(data: &Bytes) -> String {
    let mut hasher = Sha256::default();
    hasher.update(data.as_ref());
    let hash = hasher.finalize();
    base64::encode(&hash[..])
}
