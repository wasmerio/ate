use crate::bus::WasmCallerContext;
use crate::wasmer_vfs::*;

pub trait MountedFileSystem
where
    Self: FileSystem + std::fmt::Debug,
{
    fn set_ctx(&self, ctx: &WasmCallerContext);
}
