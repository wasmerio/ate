#![allow(dead_code)]
use std::collections::HashMap;

use super::*;

pub struct Client {
    pub(crate) builder: ClientBuilder,
}

impl Client {
    pub fn get<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::GET, url)
    }

    pub fn post<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::POST, url)
    }

    pub fn put<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::PUT, url)
    }

    pub fn patch<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::PATCH, url)
    }

    pub fn delete<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::DELETE, url)
    }

    pub fn head<U: IntoUrl>(self, url: U) -> RequestBuilder {
        self.request(http::Method::HEAD, url)
    }

    pub fn request<U: IntoUrl>(self, method: http::Method, url: U) -> RequestBuilder {
        RequestBuilder {
            method,
            url: url.into_url().unwrap(),
            client: self,
            headers: HashMap::default(),
            request: None,
        }
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }
}
