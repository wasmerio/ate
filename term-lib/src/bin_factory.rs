#![allow(dead_code)]
#![allow(unused)]
use bytes::Bytes;
use derivative::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::RwLock;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use sha2::{Digest, Sha256};

use super::common::*;
use super::err;
use super::fs::TmpFileSystem;
use crate::api::*;
use crate::fd::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct BinaryPackage
{
    #[derivative(Debug = "ignore")]
    pub data: Bytes,
    pub hash: String,
    pub fs: TmpFileSystem,
}

impl BinaryPackage
{
    pub fn new(data: Bytes) -> BinaryPackage {
        let hash = hash_of_binary(&data);
        BinaryPackage {
            data,
            hash,
            fs: TmpFileSystem::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinFactory {
    pub cache: Arc<RwLock<HashMap<String, Option<BinaryPackage>>>>,
}

impl BinFactory {
    pub fn new() -> BinFactory {
        BinFactory {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, cmd: &str, mut stdout: Fd) -> Option<BinaryPackage> {
        let mut cmd = cmd.to_string();

        // Fast path
        {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(&cmd) {
                return data.clone();
            }
        }

        // Tell the console we are fetching
        stdout.write_clear_line().await;
        let _ = stdout.write("Fetching...".as_bytes()).await;
        
        // Slow path
        let mut cache = self.cache.write().await;

        // Check the cache
        if let Some(data) = cache.get(&cmd) {
            stdout.write_clear_line().await;
            return data.clone();
        }

        // First just try to find it
        if let Ok(data) = fetch_file(format!("/bin/{}.wasm", cmd).as_str()).await {
            let data = BinaryPackage::new(Bytes::from(data));
            cache.insert(cmd, Some(data.clone()));
            stdout.write_clear_line().await;
            return Some(data);
        }

        // NAK
        cache.insert(cmd, None);
        stdout.write_clear_line().await;
        return None;
    }
}

async fn fetch_file(path: &str) -> Result<Vec<u8>, i32> {
    let system = System::default();
    system.fetch_file(path).await
}

fn hash_of_binary(data: &Bytes) -> String
{
    let mut hasher = Sha256::default();
    hasher.update(data.as_ref());
    let hash = hasher.finalize();
    base64::encode(&hash[..])
}