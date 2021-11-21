use bytes::{Bytes, BytesMut};

extern "C" {
    fn __prevent_drop();
}

#[repr(C)]
#[derive(Debug)]
#[must_use = "you must invoke 'manual_drop' or consume it in another way or else memory will leak"]
pub struct Buffer {
    pub(super) data: *mut u8,
    pub(super) len: usize,
}

impl Unpin for Buffer {}
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Drop
for Buffer
{
    // This code is to prevent a drop of this buffer which would
    // cause it to memory leak... instead call the manual_drop function
    fn drop(&mut self) {
        unsafe { __prevent_drop(); }
    }
}

impl Buffer
{
    pub(super) fn manual_drop(self) {
        let s = unsafe { std::slice::from_raw_parts_mut(self.data, self.len) };
        let s = s.as_mut_ptr();
        unsafe {
            Box::from_raw(s);
        }
    }
}

impl From<&[u8]>
for Buffer {
    fn from(buf: &[u8]) -> Buffer {
        let mut buf = buf.to_vec().into_boxed_slice();
        let data = buf.as_mut_ptr();
        let len = buf.len();
        std::mem::forget(buf);
        Buffer { data, len }
    }
}

impl From<Vec<u8>>
for Buffer {
    fn from(buf: Vec<u8>) -> Buffer {
        let mut buf = buf.into_boxed_slice();
        let data = buf.as_mut_ptr();
        let len = buf.len();
        std::mem::forget(buf);
        Buffer { data, len }
    }
}

impl From<Bytes>
for Buffer {
    fn from(buf: Bytes) -> Buffer {
        let buf: &[u8] = buf.as_ref();
        buf.into()
    }
}

impl From<BytesMut>
for Buffer {
    fn from(buf: BytesMut) -> Buffer {
        let buf: &[u8] = buf.as_ref();
        buf.into()
    }
}

#[allow(dead_code)]
impl Buffer
{
    pub fn as_ref<'a>(&'a self) -> &'a [u8] {
        unsafe { std::slice::from_raw_parts(self.data, self.len) }
    }

    pub fn as_mut<'a>(&'a mut self) -> &'a mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.data, self.len) }
    }

    pub fn into_vec(self) -> Vec<u8> {
        unsafe {
            Vec::from_raw_parts(self.data, self.len, self.len)
        }
    }
}