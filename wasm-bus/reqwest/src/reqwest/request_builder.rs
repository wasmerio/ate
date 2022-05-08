#![allow(dead_code)]
use http::StatusCode;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use super::*;
use crate::api::ReqwestClient;
use crate::api::Response;
use wasm_bus::abi::call;

pub struct RequestBuilder {
    pub(crate) method: http::Method,
    pub(crate) url: url::Url,
    pub(crate) client: Client,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) request: Option<Body>,
}

impl RequestBuilder {
    pub fn header<T>(mut self, header: http::header::HeaderName, value: T) -> Self
    where
        T: Into<Cow<'static, str>>,
    {
        self.headers
            .insert(header.to_string(), value.into().to_string());
        self
    }

    pub fn multipart(self, multipart: multipart::Form) -> RequestBuilder {
        let mut builder = self.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        builder.request = Some(Body::from(multipart.to_string()));
        builder
    }

    pub fn bearer_auth<T>(self, token: T) -> RequestBuilder
    where
        T: std::fmt::Display,
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
        let options = self.client.options();

        let task = ReqwestClient::new(WAPM_NAME).blocking_make(
            url,
            self.method.to_string(),
            options,
            self.headers
                .iter()
                .map(|(a, b)| (a.clone(), b.clone()))
                .collect(),
            self.request
                .iter()
                .filter_map(|a| a.as_bytes())
                .map(|a| a.to_vec())
                .next(),
        );

        let res = task.map_err(|err| err.into_io_error())?;
        let res = res.map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("syscall error - code={}", err).as_str(),
            )
        })?;

        let status = StatusCode::from_u16(res.status).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("invalid status code returned by the server - {}", err).as_str(),
            )
        })?;

        Ok(Response {
            ok: res.ok,
            redirected: res.redirected,
            status: status.as_u16(),
            status_text: res.status_text,
            headers: res.headers,
            pos: 0usize,
            data: res.data,
        })
    }
}
