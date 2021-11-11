#![allow(dead_code)]
use std::collections::HashMap;
use std::borrow::Cow;
use std::io::{Read, Write};

use crate::web_command::WebCommand;

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

        let cmd = WebCommand::WebRequest {
            url,
            method: self.method.to_string(),
            headers: self.headers.iter().map(|(a, b)| (a.clone(), b.clone())).collect(),
            body: self.request.iter().filter_map(|a| a.as_bytes()).map(|a| a.to_vec()).next()
        };
        let cmd = cmd.serialize()?;

        let mut file = std::fs::File::open("/dev/web")?;
        
        let submit = format!("{}\n", cmd);
        let _ = file.write_all(submit.as_bytes());

        let mut data = Vec::new();
        read_to_end(&mut file, &mut data)?;

        Ok(
            Response {
                pos: 0,
                data,
            }
        )
    }
}

fn read_to_end(file: &mut std::fs::File, data: &mut Vec<u8>) -> Result<(), std::io::Error>
{
    let mut buf = [0u8; 4096];
    loop {
        match file.read(&mut buf[..]) {
            Ok(read) if read == 0usize => {
                break;
            },
            Ok(read) => {
                data.extend_from_slice(&buf[..read]);
            },
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                std::thread::yield_now();
                continue;
            },
            Err(err) if err.kind() == ErrorKind::ConnectionAborted ||
                              err.kind() == ErrorKind::ConnectionReset ||
                              err.kind() == ErrorKind::BrokenPipe => {
                break;                       
            }
            Err(err) => {
                return Err(err);
            },
        }
    }
    return Ok(())
}