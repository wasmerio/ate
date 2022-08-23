use std::{sync::Arc, ops::Deref};
use webc::{FsEntryType, WebC};
use webc_vfs::VirtualFileSystem;

use serde::*;
#[allow(unused_imports)]
use tracing::{error, warn};

use crate::{api::{System, ReqwestOptions}, bin_factory::BinaryPackage};

const WAPM_WEBC_URL: &'static str = "https://registry.wapm.dev/graphql?query=";
const WAPM_WEBC_QUERY: &'static str = r#"
{
    getPackage(name: "<NAME>") {
        lastVersion {
            distribution {
                piritaDownloadUrl
            }
        }
    }
}"#;
const WAPM_WEBC_QUERY_TAG: &'static str = "<NAME>";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WapmWebQueryGetPackageLastVersionDistribution {
    #[serde(rename = "piritaDownloadUrl")]
    pub pirita_download_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WapmWebQueryGetPackageLastVersion {
    #[serde(rename = "distribution")]
    pub distribution: WapmWebQueryGetPackageLastVersionDistribution
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WapmWebQueryGetPackage {
    #[serde(rename = "lastVersion")]
    pub last_version: WapmWebQueryGetPackageLastVersion
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WapmWebQueryData {
    #[serde(rename = "getPackage")]
    pub get_package: Option<WapmWebQueryGetPackage>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WapmWebQuery {
    #[serde(rename = "data")]
    pub data: WapmWebQueryData,
}

pub(crate) async fn fetch_webc(cache_dir: Option<&str>, name: &str) -> Option<BinaryPackage> {
    let sys = System::default();

    let url = format!(
        "{}{}",
        WAPM_WEBC_URL,
        urlencoding::encode(
            WAPM_WEBC_QUERY.replace(WAPM_WEBC_QUERY_TAG,
                name.replace("\"", "'").as_str())
            .as_str()
        )
    );
    let options = ReqwestOptions::default();
    let headers = Default::default();
    let data = None;
    if let Some(wapm) = sys.reqwest(url.as_str(), "POST", options, headers, data).await {
        match wapm {
            Ok(wapm) => {
                if wapm.status == 200 {
                    if let Some(data) = wapm.data {
                        match serde_json::from_slice::<'_, WapmWebQuery>(data.as_ref()) {
                            Ok(query) => {
                                if let Some(package) = query.data.get_package {
                                    return download_webc(cache_dir, name, package.last_version.distribution.pirita_download_url).await;
                                } else {
                                    warn!("failed to parse WAPM package: {}", String::from_utf8_lossy(data.as_ref()));    
                                }
                            },
                            Err(err) => {
                                warn!("failed to deserialize WAPM response: {}", err);
                            }
                        }
                    }
                } else {
                    warn!("failed to contact WAPM: http_code={}, http_response={}", wapm.status, wapm.status_text);
                }
            },
            Err(code) => {
                warn!("failed to contact WAPM: http_code={}", code);
            }
        }
    }

    None
}

async fn download_webc(cache_dir: Option<&str>, name: &str, pirita_download_url: String) -> Option<BinaryPackage>
{
    let compute_path = |cache_dir: &str, name: &str| {
        let name = name.replace("/", "._.");
        std::path::Path::new(cache_dir).join(format!("{}.webc", name.as_str()).as_str())
    };

    // build the parse options
    let options = webc::ParseOptions::default();

    // fast path
    if let Some(cache_dir) = cache_dir {
        let path = compute_path(cache_dir, name);
        #[cfg(feature = "sys")]
        if path.exists() {
            match webc::WebCMmap::parse(path.clone(), &options) {
                Ok(webc) => {
                    unsafe {
                        let webc = Arc::new(webc);
                        return parse_webc(webc.as_webc_ref(), webc.clone());
                    }
                },
                Err(err) => {
                    warn!("failed to parse WebC: {}", err);
                }
            }
        }
        if let Ok(data) = std::fs::read(path) {
            match webc::WebCOwned::parse(data, &options) {
                Ok(webc) => {
                    unsafe {
                        let webc = Arc::new(webc);
                        return parse_webc(webc.as_webc_ref(), webc.clone());
                    }
                },
                Err(err) => {
                    warn!("failed to parse WebC: {}", err);
                }
            }
        }
    }

    // slow path
    let cache_dir = cache_dir.map(|a| a.to_string());
    let name = name.to_string();
    if let Some(data) = download_webc_miss(pirita_download_url.as_str()).await {
        if let Some(cache_dir) = cache_dir {
            let path = compute_path(cache_dir.as_str(), name.as_str());
            let _ = std::fs::create_dir_all(path.parent().unwrap().clone());
            let _ = std::fs::write(path.as_path(), &data[..]);

            #[cfg(feature = "sys")]
            match webc::WebCMmap::parse(path, &options) {
                Ok(webc) => {
                    unsafe {
                        let webc = Arc::new(webc);
                        return parse_webc(webc.as_webc_ref(), webc.clone());
                    }
                },
                Err(err) => {
                    warn!("failed to parse WebC: {}", err);
                }
            }
        }

        match webc::WebCOwned::parse(data, &options) {
            Ok(webc) => {
                unsafe {
                    let webc = Arc::new(webc);
                    return parse_webc(webc.as_webc_ref(), webc.clone());
                }
            },
            Err(err) => {
                warn!("failed to parse WebC: {}", err);
            }
        }
    }

    None
}

async fn download_webc_miss(pirita_download_url: &str) -> Option<Vec<u8>> {
    let sys = System::default();

    let mut options = ReqwestOptions::default();
    options.gzip = true;

    let headers = Default::default();
    let data = None;

    if let Some(wapm) = sys.reqwest(pirita_download_url, "GET", options, headers, data).await {
        match wapm {
            Ok(wapm) => {
                if wapm.status == 200 {
                    return wapm.data;
                } else {
                    warn!("failed to download WebC: http_code={}, http_response={}", wapm.status, wapm.status_text);
                }
            },
            Err(code) => {
                warn!("failed to download WebC: http_code={}", code);
            }
        }
    }

    None
}

unsafe fn parse_webc<'a, 'b, T>(webc: webc::WebC<'a>, ownership: Arc<T>) -> Option<BinaryPackage>
where T: std::fmt::Debug + Send + Sync + 'static,
      T: Deref<Target=WebC<'static>>
{
    let package_name = webc.get_package_name();
    let mut pck = webc.manifest.entrypoint
        .iter()
        .filter_map(|entry| {
            webc.manifest.commands.get(entry)
                .map(|a| (a, entry))
        })
        .filter_map(|(cmd, entry)| {
            let api = if cmd.runner.starts_with("https://webc.org/runner/emscripten") {
                "emscripten"
            } else if cmd.runner.starts_with("https://webc.org/runner/wasi") {
                "wasi"
            } else {
                warn!("unsupported runner - {}", cmd.runner);
                return None;
            };
            match webc.get_atom_name_for_command(api, entry.as_str()) {
                Ok(a) => Some(a),
                Err(err) => {
                    warn!("failed to find atom name for entry command({}) - {}", entry.as_str(), err);
                    None
                }
            }
        })
        .filter_map(|atom| {
            match webc.get_atom(&package_name, atom.as_str()) {
                Ok(a) => Some(a),
                Err(err) => {
                    warn!("failed to find atom for atom name({}) - {}", atom, err);
                    None
                }
            }
        })
        .map(|atom| {
            BinaryPackage::new_with_ownership(atom.into(), ownership.clone())
        })
        .next();

    if let Some(pck) = pck.as_mut() {
        let top_level_dirs = webc
            .get_volumes_for_package(&package_name)
            .into_iter()
            .flat_map(|volume| {
                webc.volumes
                    .get(&volume)
                    .unwrap()
                    .header
                    .top_level
                    .iter()
                    .filter(|e| e.fs_type == FsEntryType::Dir)
                    .map(|e| e.text.to_string())
            })
            .collect::<Vec<_>>();

        pck.webc_fs = Some(Arc::new(VirtualFileSystem::init(ownership, &package_name)));
        pck.webc_top_level_dirs = top_level_dirs;
    }

    pck
}