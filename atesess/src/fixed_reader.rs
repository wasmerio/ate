use ate::comms::StreamReader;
use std::io::*;
use ate::prelude::EncryptKey;

pub struct FixedReader
{
    data: Option<Vec<u8>>,
}

impl FixedReader
{
    pub fn new(data: Vec<u8>) -> FixedReader
    {
        FixedReader {
            data: Some(data)
        }
    }
}

#[async_trait::async_trait]
impl StreamReader
for FixedReader
{
    async fn read_buf_with_header(&mut self, _wire_encryption: &Option<EncryptKey>, total_read: &mut u64) -> Result<Vec<u8>>
    {
        match self.data.take() {
            Some(data) => {
                *total_read += data.len() as u64;
                return Ok(data);
            }
            None => {
                return Ok(Vec::new());
            }
        }
    }
}