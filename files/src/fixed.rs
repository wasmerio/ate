#![allow(unused_imports)]
use super::api::FileKind;
use super::model::*;
use crate::api::FileApi;
use async_trait::async_trait;
use ate::prelude::PrimaryKey;
use serde::*;

#[derive(Debug, Clone)]
pub struct FixedFile {
    ino: u64,
    #[allow(dead_code)]
    kind: FileKind,
    uid: u32,
    gid: u32,
    size: u64,
    mode: u32,
    name: String,
    created: u64,
    updated: u64,
}

impl FixedFile {
    pub fn new(ino: u64, name: String, kind: FileKind) -> FixedFile {
        FixedFile {
            ino,
            kind,
            uid: 0,
            gid: 0,
            size: 0,
            mode: 0,
            name: name,
            created: 0,
            updated: 0,
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

    #[allow(dead_code)]
    pub fn created(mut self, val: u64) -> FixedFile {
        self.created = val;
        self
    }

    #[allow(dead_code)]
    pub fn updated(mut self, val: u64) -> FixedFile {
        self.updated = val;
        self
    }
}

#[async_trait]
impl FileApi for FixedFile {
    fn kind(&self) -> FileKind {
        FileKind::FixedFile
    }

    fn ino(&self) -> u64 {
        self.ino
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
