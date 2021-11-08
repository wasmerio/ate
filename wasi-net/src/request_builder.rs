#![allow(dead_code)]
use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{Read, Write};

use super::*;

pub struct RequestBuilder {
    pub(crate) method: http::Method,
    pub(crate) url: url::Url,
    pub(crate) client: Client,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) request: Option<Body>,
}

impl RequestBuilder {
    pub fn header<T>(mut self, header: http::header::HeaderName, value: T) -> Self
    where T: Into<Cow<'static, str>>
    {
        self.headers.insert(header.to_string(), value.into().to_string());
        self
    }

    pub fn multipart(self, multipart: multipart::Form) -> RequestBuilder {
        let mut builder = self.header(
            header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        );
        builder.request = Some(Body::from(multipart.to_string()));
        builder
    }

    pub fn bearer_auth<T>(self, token: T) -> RequestBuilder
    where T: std::fmt::Display,
    {
        let token = format!("{}", token);
        if token.len() <= 0 {
            return self;
        }
        let header_value = format!("Bearer {}", token);
        self.header(header::AUTHORIZATION, header_value)
    }

    pub fn send(self) -> Result<Response, std::io::Error> {
        let url = self.url.to_string();

        let headers = self.headers.iter().map(|(a, b)| (a.clone(), b.clone())).collect::<Vec<_>>();
        let headers = serde_json::to_vec(&headers).unwrap();
        let headers = base64::encode(&headers[..]);

        let data = if let Some(request) = self.request {
            if let Some(request) = request.as_bytes() {
                base64::encode(request)
            } else {
                String::new()    
            }
        } else {
            String::new()
        };

        let submit_url = format!("{}\n", url);
        let submit_method = format!("{}\n", self.method.to_string());
        let submit_headers = format!("{}\n", headers);
        let submit_data = format!("{}\n", data);
        
        let mut file = std::fs::File::open("/dev/web")?;

        let _ = file.write_all(submit_url.as_bytes());
        let _ = file.write_all(submit_method.as_bytes());
        let _ = file.write_all(submit_headers.as_bytes());
        let _ = file.write_all(submit_data.as_bytes());

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        Ok(
            Response {
                pos: 0,
                data,
            }
        )
    }
}