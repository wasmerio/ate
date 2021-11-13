#![allow(dead_code)]
use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{Write};
use http::StatusCode;

use crate::web_command::WebCommand;
use crate::web_response::WebResponse;

use super::*;
use super::utils::*;

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

        let cmd = WebCommand::WebRequestVersion1 {
            url,
            method: self.method.to_string(),
            headers: self.headers.iter().map(|(a, b)| (a.clone(), b.clone())).collect(),
            body: self.request.iter().filter_map(|a| a.as_bytes()).map(|a| a.to_vec()).next()
        };
        let cmd = cmd.serialize()?;

        let mut file = std::fs::File::open("/dev/web")?;
        
        let submit = format!("{}\n", cmd);
        let _ = file.write_all(submit.as_bytes());

        let res = read_response(&mut file)?;
        let (
            ok,
            redirected,
            status,
            status_text,
            headers,
            has_data
        ) = match res {
            WebResponse::Error { msg } => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, msg.as_str()));
            },
            WebResponse::WebSocketVersion1 { .. } => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "server returned a web socket instead of a web request"));
            },
            WebResponse::WebRequestVersion1 {
                ok,
                redirected,
                status,
                status_text,
                headers,
                has_data
            } => {
                (ok, redirected, status, status_text, headers, has_data)
            }
        };
    
        let status = StatusCode::from_u16(status).map_err(|err| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("invalid status code returned by the server - {}", err).as_str())
        })?;
        
        let data = if has_data {
            let mut data = Vec::new();
            read_to_end(&mut file, &mut data)?;
            Some(data)
        } else {
            None
        };

        Ok(
            Response {
                ok,
                redirected,
                status,
                status_text,
                headers,
                pos: data.as_ref().map_or_else(|| 0usize, |a| a.len()),
                data,
            }
        )
    }
}