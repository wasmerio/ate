use http::header::HeaderName;
use http::HeaderMap;
use http::HeaderValue;
use http::StatusCode;
use serde::*;
use serde::de::DeserializeOwned;
use std::io::Read;
use std::io::Error;
use std::io::ErrorKind;

use crate::backend::reqwest::*;

impl Response {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Error> {
        match self.data.as_ref() {
            Some(data) => serde_json::from_slice(&data[..]).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!(
                        "failed to deserialize ({} bytes) into json - {}",
                        data.len(),
                        e
                    )
                    .as_str(),
                )
            }),
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
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
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK)
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