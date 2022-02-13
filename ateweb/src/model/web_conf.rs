use fxhash::FxHashMap;
use serde::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebConf {
    /// Forces the host to be redirected to a new URL
    #[serde(default)]
    pub redirect: Option<String>,
    /// Hosts the ATE web sockets on this site
    #[serde(default)]
    pub ate_proxy: bool,
    /// Enable COOP (Cross-Origin-Opener-Policy: same-origin)
    #[serde(default)]
    pub coop: bool,
    /// Force (by direction) all requests to HTTPS
    #[serde(default)]
    pub force_https: bool,
    /// The default page
    #[serde(default)]
    pub default_page: Option<String>,
    /// Redirects certain status codes to specific pages
    #[serde(default)]
    pub status_pages: FxHashMap<u16, String>,
    /// List of the domains that this domain will reverse proxy for cors
    #[serde(default)]
    pub cors_proxy: Vec<String>,
}

impl Default for WebConf {
    fn default() -> Self {
        WebConf {
            redirect: None,
            coop: false,
            ate_proxy: false,
            force_https: false,
            default_page: None,
            status_pages: FxHashMap::default(),
            cors_proxy: Vec::new(),
        }
    }
}
