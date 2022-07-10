#![allow(dead_code)]
#![allow(unused)]
#[cfg(feature = "cached_compiling")]
use crate::wasmer::Module;
#[cfg(feature = "cached_compiling")]
use crate::wasmer::Store;
use bytes::Bytes;
use derivative::*;
use serde::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};

use crate::eval::Compiler;
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
    pub wapm: Option<String>,
    pub base_dir: Option<String>,
    pub fs: TmpFileSystem,
    pub mappings: Vec<String>,
    pub envs: HashMap<String, String>,
}

impl BinaryPackage {
    pub fn new(data: Bytes) -> BinaryPackage {
        let forced_exit = Arc::new(AtomicU32::new(0));
        let hash = hash_of_binary(&data);
        BinaryPackage {
            data,
            hash,
            chroot: false,
            wapm: None,
            base_dir: None,
            fs: TmpFileSystem::new(),
            mappings: Vec::new(),
            envs: HashMap::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliasConfig {
    pub run: String,
    #[serde(default)]
    pub chroot: bool,
    #[serde(default)]
    pub wapm: Option<String>,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub mappings: Vec<String>,
    #[serde(default)]
    pub envs: HashMap<String, String>,
}

#[derive(Debug)]
#[cfg(feature = "cached_compiling")]
pub struct CachedCompiledModules {
    modules: RwLock<HashMap<String, Option<Module>>>,
    cache_dir: Option<String>,
}

#[cfg(feature = "cached_compiling")]
impl CachedCompiledModules
{
    pub fn new(cache_dir: Option<String>) -> CachedCompiledModules {
        let cache_dir = cache_dir.map(|a| shellexpand::tilde(&a).to_string());
        CachedCompiledModules {
            modules: RwLock::new(HashMap::default()),
            cache_dir,
        }
    }

    pub async fn get_compiled_module(&self, data_hash: &String, compiler: Compiler) -> Option<Module> {
        let key = format!("{}-{}", data_hash, compiler);

        // fast path
        {
            let cache = self.modules.read().await;
            if let Some(module) = cache.get(&key).map(|a| a.clone()).flatten() {
                return Some(module);
            }
        }

        // slow path
        let mut cache = self.modules.write().await;
        if let Some(module) = cache.get(&key).map(|a| a.clone()).flatten() {
            return Some(module);
        }

        // Attempt to read it from the cache directory and populate the cache
        if let Some(cache_dir) = &self.cache_dir {
            unsafe {
                let store = compiler.new_store();
                let path = std::path::Path::new(cache_dir.as_str()).join(format!("{}.bin", key).as_str());
                if let Ok(data) = std::fs::read(path) {
                    let mut decoder = weezl::decode::Decoder::new(weezl::BitOrder::Msb, 8);
                    if let Ok(data) = decoder.decode(&data[..]) {
                        if let Ok(module) = Module::deserialize(&store, &data[..]) {
                            cache.insert(key.clone(), Some(module.clone()));
                            return Some(module);
                        }
                    }
                }
            }
        }

        // Not found
        return None;
    }

    pub async fn set_compiled_module(&self, data_hash: String, compiler: Compiler, compiled_module: Module) {
        let key = format!("{}-{}", data_hash, compiler);

        // Attempt to insert it
        {
            let mut cache = self.modules.write().await;
            if cache.contains_key(&key) {
                return;
            }
            cache.insert(key.clone(), Some(compiled_module.clone()));
        }

        // If its inserted then we should try and update the cache directory
        if let Some(cache_dir) = &self.cache_dir {
            let path = std::path::Path::new(cache_dir.as_str()).join(format!("{}.bin", key).as_str());
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
            if let Ok(data) = compiled_module.serialize() {
                let mut encoder = weezl::encode::Encoder::new(weezl::BitOrder::Msb, 8);
                if let Ok(data) = encoder.encode(&data[..]) {
                    let _ = std::fs::write(path, &data[..]);
                }
            }
        }
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

    pub async fn clear(&self) {
        self.wax.lock().unwrap().clear();
        self.alias.write().await.clear();
        self.cache.write().await.clear();
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
        #[cfg(target_family = "wasm")]
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
            #[cfg(target_family = "wasm")]
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return data.clone();
        }

        // First just try to find it
        if let Ok(data) = fetch_file(format!("/bin/{}.wasm", name).as_str())
            .await
            .unwrap()
        {
            let data = BinaryPackage::new(Bytes::from(data));
            cache.insert(name, Some(data.clone()));
            #[cfg(target_family = "wasm")]
            if stderr.is_tty() {
                stderr.write_clear_line().await;
            }
            return Some(data);
        }

        // NAK
        cache.insert(name, None);
        #[cfg(target_family = "wasm")]
        if stderr.is_tty() {
            stderr.write_clear_line().await;
        }
        return None;
    }

    #[cfg(feature = "cached_compiling")]
    pub async fn get_compiled_module(&self, data_hash: &String, compiler: Compiler) -> Option<Module> {
        self.compiled_modules.get_compiled_module(data_hash, compiler).await
    }

    #[cfg(feature = "cached_compiling")]
    pub async fn set_compiled_module(&self, data_hash: String, compiler: Compiler, compiled_module: Module) {
        self.compiled_modules
            .set_compiled_module(data_hash, compiler, compiled_module)
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
        if let Ok(data) = fetch_file(alias_path.as_str()).await.unwrap() {
            // Decode the file into a yaml configuration
            match serde_yaml::from_slice::<AliasConfig>(&data[..]) {
                Ok(alias) => {
                    debug!("binary alias '{}' found for {}", alias.run, name);
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
    hex::encode(&hash[..])
}
