use super::*;

#[repr(C)]
#[derive(Debug)]
pub struct BString {
    buf: Buffer,
}

impl From<String>
for BString
{
    fn from(input: String) -> BString {
        let buf = input.as_bytes();
        BString {
            buf: buf.into()
        }
    }
}

impl From<&str>
for BString
{
    fn from(input: &str) -> BString {
        let buf = input.as_bytes();
        BString {
            buf: buf.into()
        }
    }
}

#[allow(dead_code)]
impl BString
{
    pub fn as_ref<'a>(&'a self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.buf.data, self.buf.len) }
    }

    pub fn as_str<'a>(&'a self) -> &'a str {
        unsafe { std::str::from_utf8_unchecked(self.as_ref()) }
    }
}