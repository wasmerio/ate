#![allow(dead_code)]
#![allow(unused)]
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::result::Result as StdResult;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
#[allow(unused_imports, dead_code)]
use tracing::{debug, error, info, trace, warn};
use std::sync::atomic::AtomicU32;

use crate::wasmer_vfs::Result as FsResult;
use crate::wasmer_vfs::*;
use crate::wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};
use crate::wasmer_vfs::mem_fs;

use super::api::*;
use crate::bus::WasmCallerContext;
use crate::fd::*;
use crate::stdio::*;
use crate::tty::*;

#[derive(Debug, Clone)]
pub struct TmpFileSystem
{
    fs: mem_fs::FileSystem,
}

impl TmpFileSystem
{
    pub fn new() -> Self {
        Self {
            fs: mem_fs::FileSystem::default(),
        }
    }
}

impl MountedFileSystem
for TmpFileSystem
{
    fn set_ctx(&self, ctx: &WasmCallerContext) {
    }
}


impl FileSystem
for TmpFileSystem
{
    fn read_dir(&self, path: &Path) -> Result<ReadDir>
    {
        self.fs.read_dir(path)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        self.fs.create_dir(path)
    }
    
    fn remove_dir(&self, path: &Path) -> Result<()> {
        self.fs.remove_dir(path)
    }
    
    fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.fs.rename(from, to)
    }
    
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        self.fs.metadata(path)
    }
    
    fn symlink_metadata(&self, path: &Path) -> Result<Metadata> {
        self.fs.symlink_metadata(path)
    }
    
    fn remove_file(&self, path: &Path) -> Result<()> {
        self.fs.remove_file(path)
    }

    fn new_open_options(&self) -> OpenOptions {
        self.fs.new_open_options()
    }
}