use std::io::Read;
use serde::de::DeserializeOwned;

pub struct Response {
    pub(crate) pos: usize,
    pub(crate) data: Vec<u8>,
}

impl Response
{
    pub fn json<T: DeserializeOwned>(self) -> Result<T, crate::Error> {
        serde_json::from_slice(&self.data[..])
            .map_err(|e| crate::Error::new(crate::ErrorKind::Other, 
                format!("failed to deserialize ({} bytes) into json - {}", self.data.len(), e).as_str()))
    }
}

impl Read
for Response
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0usize);
        }
        let remaining = &self.data[self.pos..];
        let sub = remaining.len().min(buf.len());
        buf[0..sub].clone_from_slice(&remaining[0..sub]);
        self.pos += sub;
        Ok(sub)
    }
}