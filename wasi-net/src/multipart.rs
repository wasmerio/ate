use std::borrow::Cow;
use urlencoding::encode;

pub struct Form {
    parts: Vec<(String, String)>
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

impl Form {
    /// Creates a new Form without any content.
    pub fn new() -> Form {
        Form {
            parts: Vec::new(),
        }
    }

    pub fn text<T, U>(mut self, name: T, value: U) -> Form
    where
        T: Into<Cow<'static, str>>,
        U: Into<Cow<'static, str>>,
    {
        self.parts.push((name.into().to_string(), value.into().to_string()));
        self
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        for n in 0..self.parts.len() {
            let first = n == 0;
            if first == false {
                ret += "&";
            }
            let (name, value) = &self.parts[n];
            ret += &encode(name.as_str());
            ret += "=";
            ret += &encode(value.as_str());
        }
        ret
    }
}