use serde::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
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