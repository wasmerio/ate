pub struct ReqwestResponse {
    pub ok: bool,
    pub redirected: bool,
    pub status: u16,
    pub status_text: String,
    pub data: Vec<u8>,
}
