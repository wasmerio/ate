#![allow(unused_imports)]
use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;
use ate::prelude::PrimaryKey;
use super::api::SpecType;

#[derive(Debug, Clone)]
pub struct FixedFile
{
    ino: u64,
    kind: FileType,
    uid: u32,
    gid: u32,
    size: u64,
    mode: u32,
    name: String,
}

impl FixedFile
{
    pub fn new(key: &PrimaryKey, name: String, kind: FileType) -> FixedFile {
        FixedFile {
            ino: key.as_u64(),
            kind,
            uid: 0,
            gid: 0,
            size: 0,
            mode: 0,
            name: name,
        }
    }

    pub fn uid(mut self, val: u32) -> FixedFile {
        self.uid = val;
        self
    }

    pub fn gid(mut self, val: u32) -> FixedFile {
        self.gid = val;
        self
    }

    #[allow(dead_code)]
    pub fn mode(mut self, val: u32) -> FixedFile {
        self.mode = val;
        self
    }

    #[allow(dead_code)]
    pub fn size(mut self, val: u64) -> FixedFile {
        self.size = val;
        self
    }
}

impl FileApi
for FixedFile
{
    fn spec(&self) -> SpecType {
        SpecType::FixedFile
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn kind(&self) -> FileType {
        self.kind
    }

    fn uid(&self) -> u32 {
        self.uid
    }

    fn gid(&self) -> u32 {
        self.gid
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn mode(&self) -> u32 {
        self.mode
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}