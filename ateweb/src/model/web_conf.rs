use serde::*;
use fxhash::FxHashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebConf
{
    /// Forces the host to be redirected to a new URL
    #[serde(default)]
    pub redirect: Option<String>,
    /// Hosts the ATE web sockets on this site
    #[serde(default)]
    pub ate_proxy: bool,
    /// The default page
    #[serde(default)]
    pub default_page: Option<String>,
    /// Redirects certain status codes to specific pages
    #[serde(default)]
    pub status_pages: FxHashMap<u16, String>,
}

impl Default
for WebConf
{
    fn default() -> Self
    {
        WebConf {
            redirect: None,
            ate_proxy: false,
            default_page: None,
            status_pages: FxHashMap::default(),
        }
    }
}