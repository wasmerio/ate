use http::header::HeaderName;
use http::HeaderMap;
use http::HeaderValue;
use http::StatusCode;
use serde::de::DeserializeOwned;
use std::io::Read;

pub struct Response {
    pub(crate) pos: usize,
    pub(crate) data: Option<Vec<u8>>,
    pub(crate) ok: bool,
    pub(crate) redirected: bool,
    pub(crate) status: StatusCode,
    pub(crate) status_text: String,
    pub(crate) headers: Vec<(String, String)>,
}

impl Response {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, crate::Error> {
        match self.data.as_ref() {
            Some(data) => serde_json::from_slice(&data[..]).map_err(|e| {
                crate::Error::new(
                    crate::ErrorKind::Other,
                    format!(
                        "failed to deserialize ({} bytes) into json - {}",
                        data.len(),
                        e
                    )
                    .as_str(),
                )
            }),
            None => {
                return Err(crate::Error::new(
                    crate::ErrorKind::Other,
                    format!("failed to deserialize into json - no data was returned by the server")
                        .as_str(),
                ));
            }
        }
    }

    pub fn content_length(&self) -> Option<u64> {
        self.data.as_ref().map(|a| a.len() as u64)
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn ok(&self) -> bool {
        self.ok
    }

    pub fn redirected(&self) -> bool {
        self.redirected
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    pub fn headers(&self) -> HeaderMap {
        let mut ret = HeaderMap::new();
        for (header, value) in self.headers.iter() {
            let val = match HeaderValue::from_str(value) {
                Ok(a) => a,
                Err(_) => {
                    continue;
                }
            };
            let parsed: HeaderName = match header.parse() {
                Ok(a) => a,
                Err(_) => {
                    continue;
                }
            };
            ret.append(parsed, val);
        }
        ret
    }
}

impl Read for Response {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.data.as_ref() {
            Some(data) => {
                if self.pos >= data.len() {
                    return Ok(0usize);
                }
                let remaining = &data[self.pos..];
                let sub = remaining.len().min(buf.len());
                buf[0..sub].clone_from_slice(&remaining[0..sub]);
                self.pos += sub;
                Ok(sub)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "The server returned no data",
            )),
        }
    }
}
