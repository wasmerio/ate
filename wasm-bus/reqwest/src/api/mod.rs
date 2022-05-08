use serde::*;
use wasm_bus::macros::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReqwestOptions
{
    pub gzip: bool,
    pub cors_proxy: Option<String>,
}

#[wasm_bus(format = "bincode")]
pub trait Reqwest {
    async fn make(
        &self,
        url: String,
        method: String,
        options: ReqwestOptions,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
    ) -> Result<Response, i32>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub pos: usize,
    pub data: Option<Vec<u8>>,
    pub ok: bool,
    pub redirected: bool,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
}
