#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use std::sync::Arc;
use std::collections::HashMap;
use std::collections::HashSet;
use tokio::sync::RwLock;
use tokio::sync::oneshot;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, HtmlElement, HtmlInputElement, Worker};
use web_sys::{Request, RequestInit, RequestMode, Response};
use bytes::Bytes;
use sha2::{Sha256, Digest};

use super::err;
use super::fs::TmpFileSystem;
use super::common::*;

#[derive(Debug, Clone)]
pub struct BinFactory
{
    pub alias: Arc<RwLock<HashMap<String, Option<String>>>>,
    pub cache: Arc<RwLock<HashMap<String, Option<Bytes>>>>,
    pub pfs: Arc<RwLock<HashMap<String, TmpFileSystem>>>,
}

impl BinFactory
{
    pub fn new() -> BinFactory
    {
        BinFactory
        {
            alias: Arc::new(RwLock::new(HashMap::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            pfs: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub async fn get(&self, cmd: &str) -> Option<Bytes>
    {
        let mut already = HashSet::<String>::default();
        let mut cmd = cmd.to_string();

        // Fast path
        {
            let alias = self.alias.read().await;
            while let Some(Some(data))  = alias.get(&cmd) {
                if already.contains(&cmd) {
                    return None;
                }
                already.insert(cmd.clone());
                cmd = data.clone();
            }
        }
        {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(&cmd) {
                return data.clone();
            }
        }

        // Slow path
        let mut alias = self.alias.write().await;
        let mut cache = self.cache.write().await;

        // Enter a loop that will iterate the finding of this binary
        loop {
            // Infinite loop check
            if already.contains(&cmd) {
                return None;
            }
            already.insert(cmd.clone());

            // Check the cache
            if let Some(Some(data))  = alias.get(&cmd) {
                cmd = data.clone();
                continue;
            }
            if let Some(data) = cache.get(&cmd) {
                return data.clone();
            }

            // First just try to find it
            if let Ok(data) = fetch_file(format!("/bin/{}.wasm", cmd).as_str()).await {
                let data = Bytes::from(data);
                cache.insert(cmd, Some(data.clone()));
                return Some(data);
            }

            // Check for an alias
            if let Ok(data) = fetch_file(format!("/bin/{}.alias", cmd).as_str()).await {
                let next = String::from_utf8_lossy(&data[..]).into_owned().trim().to_string();
                debug!("binary alias '{}' found for {}", next, cmd);
                alias.insert(cmd, Some(next.clone()));
                cmd = next;
                continue;
            }

            // NAK
            alias.insert(cmd.clone(), None);
            cache.insert(cmd, None);
            return None;
        }
    }

    pub async fn fs(&self, binary: &Bytes) -> TmpFileSystem
    {
        let mut hasher = Sha256::default();
        hasher.update(binary.as_ref());
        let hash = hasher.finalize();
        let hash = base64::encode(&hash[..]);

        let mut pfs = self.pfs.write().await;
        if let Some(fs) = pfs.get(&hash) {
            return fs.clone();
        }
        let fs = TmpFileSystem::default();
        pfs.insert(hash, fs.clone());
        fs
    }
}

async fn fetch_file(cmd: &str) -> Result<Vec<u8>, i32>
{
    let cmd = cmd.to_string();
    let headers = vec![("Accept".to_string(), "application/wasm".to_string())];
    let (tx, rx) = oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        tx.send(fetch_data(cmd.as_str(), "GET", headers, None).await);
    });
    rx.await
        .map_err(|_| err::ERR_EIO)?
}