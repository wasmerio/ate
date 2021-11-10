#![allow(dead_code)]
#![allow(unused)]
#[allow(unused_imports, dead_code)]
use tracing::{info, error, debug, trace, warn};
use wasmer_wasi::vfs::*;
use wasmer_wasi::vfs::Result as FsResult;
use std::io::{self};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use std::result::Result as StdResult;
use wasmer_wasi::{types as wasi_types, WasiFile, WasiFsError};
use wasmer_wasi::vfs::{VirtualFile, FileDescriptor};

use crate::stdio::*;
use crate::fd::*;
use crate::tty::*;

pub use wasmer_wasi::vfs::mem_fs::FileSystem as TmpFileSystem;