#![allow(dead_code)]
use super::api::FileKind;
use super::model::*;
use crate::api::FileApi;
use async_trait::async_trait;
use ate::prelude::*;
use bytes::Bytes;
use error_chain::bail;
use fxhash::FxHashMap;
use seqlock::SeqLock;
use std::io::Cursor;
use std::ops::Deref;
use tokio::sync::Mutex;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, span, trace, warn, Level};

use crate::error::*;

const CACHED_BUNDLES: usize = 10; // Number of cached bundles per open file
const CACHED_PAGES: usize = 80; // Number of cached pages per open file
const ZERO_PAGE: [u8; super::model::PAGE_SIZE] = [0 as u8; super::model::PAGE_SIZE]; // Page full of zeros

pub struct RegularFile {
    pub ino: u64,
    pub created: u64,
    pub updated: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub name: String,
    pub size: SeqLock<u64>,
    pub state: Mutex<FileState>,
}

impl RegularFile {
    pub async fn new(inode: Dao<Inode>, created: u64, updated: u64) -> RegularFile {
        RegularFile {
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            size: SeqLock::new(inode.size),
            created,
            updated,
            state: Mutex::new(FileState::Immutable {
                inode,
                bundles: Box::new(array_init::array_init(|_| None)),
                pages: Box::new(array_init::array_init(|_| None)),
            }),
        }
    }

    pub async fn new_mut(inode: DaoMut<Inode>, created: u64, updated: u64) -> RegularFile {
        RegularFile {
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            size: SeqLock::new(inode.size),
            created,
            updated,
            state: Mutex::new(FileState::Mutable {
                inode,
                dirty: false,
                bundles: Box::new(array_init::array_init(|_| None)),
                pages: Box::new(array_init::array_init(|_| None)),
            }),
        }
    }
}

pub enum FileState {
    Immutable {
        inode: Dao<Inode>,
        bundles: Box<[Option<Dao<PageBundle>>; CACHED_BUNDLES]>,
        pages: Box<[Option<Dao<Page>>; CACHED_PAGES]>,
    },
    Mutable {
        dirty: bool,
        inode: DaoMut<Inode>,
        bundles: Box<[Option<DaoMut<PageBundle>>; CACHED_BUNDLES]>,
        pages: Box<[Option<DaoMutGuardOwned<Page>>; CACHED_PAGES]>,
    },
}

impl FileState {
    fn __inode(&self) -> &Inode {
        match self {
            FileState::Immutable {
                inode,
                bundles: _,
                pages: _,
            } => inode.deref(),
            FileState::Mutable {
                dirty: _,
                inode,
                bundles: _,
                pages: _,
            } => inode.deref(),
        }
    }

    pub fn get_size(&self) -> Result<u64> {
        Ok(self.__inode().size)
    }

    pub fn set_size(&mut self, val: u64) -> Result<()> {
        let (dirty, inode, _, _) = match self {
            FileState::Mutable {
                dirty,
                inode,
                bundles,
                pages,
            } => (dirty, inode, bundles, pages),
            FileState::Immutable {
                inode: _,
                bundles: _,
                pages: _,
            } => {
                bail!(FileSystemErrorKind::NoAccess);
            }
        };

        inode.as_mut().size = val;
        *dirty = true;
        Ok(())
    }

    pub async fn read_page(
        &mut self,
        mut offset: u64,
        mut size: u64,
        ret: &mut Cursor<&mut Vec<u8>>,
    ) -> Result<()> {
        // Compute the strides
        let stride_page = super::model::PAGE_SIZE as u64;
        let stride_bundle = super::model::PAGES_PER_BUNDLE as u64 * stride_page;

        // First we index into the right bundle
        let index = offset / stride_bundle;
        if index >= self.__inode().bundles.len() as u64 {
            FileState::write_zeros(ret, size).await?;
            return Ok(());
        }
        let bundle = self.__inode().bundles[index as usize];
        offset = offset - (index * stride_bundle);

        // If its a hole then just write zeros
        let bundle = match bundle {
            Some(a) => a,
            None => {
                FileState::write_zeros(ret, size).await?;
                return Ok(());
            }
        };

        // There is code for read-only inodes ... and code for writable inodes
        match self {
            FileState::Mutable {
                dirty: _,
                inode,
                bundles,
                pages,
            } => {
                // Use the cache-line to load the bundle
                let dio = inode.trans();
                let cache_index = bundle.as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut bundles[cache_index];
                let bundle = match cache_line {
                    Some(b) if *b.key() == bundle => {
                        // Cache-hit
                        b
                    }
                    _ => {
                        // Cache-miss - load the bundle into the cacheline and return its pages
                        let dao = dio.load::<PageBundle>(&bundle).await?;
                        cache_line.replace(dao);
                        cache_line.as_mut().unwrap()
                    }
                };

                // Next we index into the right page
                let index = offset / stride_page;
                if index >= bundle.pages.len() as u64 {
                    FileState::write_zeros(ret, size).await?;
                    return Ok(());
                }
                let page = bundle.pages[index as usize];
                offset = offset - (index * stride_page);

                // If its a hole then just write zeros
                let page = match page {
                    Some(a) => a,
                    None => {
                        FileState::write_zeros(ret, size).await?;
                        return Ok(());
                    }
                };

                // Use the cache-line to load the page
                let cache_index = page.as_u64() as usize % CACHED_PAGES;
                let cache_line = &mut pages[cache_index];
                let page = match cache_line {
                    Some(b) if *b.key() == page => {
                        // Cache-hit
                        b
                    }
                    _ => {
                        // Cache-miss - load the bundle into the cacheline and return its pages
                        let dao = dio.load::<Page>(&page).await?;
                        cache_line.replace(dao.as_mut_owned());
                        cache_line.as_mut().unwrap()
                    }
                };

                // Read the bytes from the page
                let buf = &page.buf;
                let sub_next = size.min(buf.len() as u64 - offset);
                if sub_next > 0 {
                    let mut reader =
                        Cursor::new(&buf[offset as usize..(offset + sub_next) as usize]);
                    tokio::io::copy(&mut reader, ret).await?;
                    size = size - sub_next;
                }
            }
            FileState::Immutable {
                inode,
                bundles,
                pages,
            } => {
                // Use the cache-line to load the bundle
                let dio = inode.dio();
                let cache_index = bundle.as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut bundles[cache_index];
                let bundle = match cache_line {
                    Some(b) if *b.key() == bundle => {
                        // Cache-hit
                        b
                    }
                    _ => {
                        // Cache-miss - load the bundle into the cacheline and return its pages
                        let dao = dio.load::<PageBundle>(&bundle).await?;
                        cache_line.replace(dao);
                        cache_line.as_mut().unwrap()
                    }
                };

                // Next we index into the right page
                let index = offset / stride_page;
                if index >= bundle.pages.len() as u64 {
                    FileState::write_zeros(ret, size).await?;
                    return Ok(());
                }
                let page = bundle.pages[index as usize];
                offset = offset - (index * stride_page);

                // If its a hole then just write zeros
                let page = match page {
                    Some(a) => a,
                    None => {
                        FileState::write_zeros(ret, size).await?;
                        return Ok(());
                    }
                };

                // Use the cache-line to load the page
                let cache_index = page.as_u64() as usize % CACHED_PAGES;
                let cache_line = &mut pages[cache_index];
                let page = match cache_line {
                    Some(b) if *b.key() == page => {
                        // Cache-hit
                        b
                    }
                    _ => {
                        // Cache-miss - load the bundle into the cacheline and return its pages
                        let dao = dio.load::<Page>(&page).await?;
                        cache_line.replace(dao);
                        cache_line.as_mut().unwrap()
                    }
                };

                // Read the bytes from the page
                let buf = &page.buf;
                let sub_next = size.min(buf.len() as u64 - offset);
                if sub_next > 0 {
                    let mut reader =
                        Cursor::new(&buf[offset as usize..(offset + sub_next) as usize]);
                    tokio::io::copy(&mut reader, ret).await?;
                    size = size - sub_next;
                }
            }
        };

        // Finish off the last bit with zeros and return the result
        FileState::write_zeros(ret, size).await?;
        Ok(())
    }

    pub async fn write_zeros(ret: &mut Cursor<&mut Vec<u8>>, size: u64) -> Result<()> {
        if size <= 0 {
            return Ok(());
        }

        let offset = super::model::PAGE_SIZE as u64 - size;
        let mut reader = Cursor::new(&ZERO_PAGE);
        reader.set_position(offset);

        tokio::io::copy(&mut reader, ret).await?;
        Ok(())
    }

    pub async fn write_page(&mut self, mut offset: u64, reader: &mut Cursor<&[u8]>) -> Result<()> {
        let (dirty, inode, bundles, pages) = match self {
            FileState::Mutable {
                dirty,
                inode,
                bundles,
                pages,
            } => (dirty, inode, bundles, pages),
            FileState::Immutable {
                inode: _,
                bundles: _,
                pages: _,
            } => {
                bail!(FileSystemErrorKind::NoAccess);
            }
        };

        *dirty = true;

        // Compute the strides
        let dio = inode.trans();
        let inode_key = inode.key().clone();
        let stride_page = super::model::PAGE_SIZE as u64;
        let stride_bundle = super::model::PAGES_PER_BUNDLE as u64 * stride_page;

        // Expand the bundles until we have enough of them to cover this write offset
        let index = offset / stride_bundle;
        if inode.bundles.len() <= index as usize {
            let mut guard = inode.as_mut();
            while guard.bundles.len() <= index as usize {
                guard.bundles.push(None);
            }
            drop(guard);
            dio.commit().await?;
        }
        offset = offset - (index * stride_bundle);

        // If the bundle is a hole then we need to fill it
        let bundle = match inode.bundles[index as usize] {
            Some(a) => a,
            None => {
                // Create the bundle
                let mut bundle = dio.store(PageBundle { pages: Vec::new() })?;
                bundle.attach_orphaned(&inode_key)?;

                let key = bundle.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = bundle.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut bundles[cache_index];
                cache_line.replace(bundle);

                // Write the entry to the inode and return its reference
                inode.as_mut().bundles.insert(index as usize, Some(key));
                dio.commit().await?;
                key
            }
        };

        // Use the cache-line to load the bundle
        let cache_index = bundle.as_u64() as usize % CACHED_BUNDLES;
        let cache_line = &mut bundles[cache_index];
        let bundle = match cache_line {
            Some(b) if *b.key() == bundle => {
                // Cache-hit
                b
            }
            _ => {
                // Cache-miss - load the bundle into the cacheline and return its pages
                let dao = dio.load::<PageBundle>(&bundle).await?;
                cache_line.replace(dao);
                cache_line.as_mut().unwrap()
            }
        };

        // Expand the page until we have enough of them to cover this write offset
        let index = offset / stride_page;
        if bundle.pages.len() <= index as usize {
            let mut guard = bundle.as_mut();
            while guard.pages.len() <= super::model::PAGES_PER_BUNDLE as usize {
                guard.pages.push(None);
            }
            while guard.pages.len() <= index as usize {
                guard.pages.push(None);
            }
            drop(guard);
            dio.commit().await?;
        }
        offset = offset - (index * stride_page);

        // If the page is a hole then we need to fill it
        let bundle_key = bundle.key().clone();
        let page_ref = bundle.pages[index as usize];
        let page = match page_ref {
            Some(a) => a.clone(),
            None => {
                // Create the page (and commit it for reference integrity)
                let mut page = dio.store(Page { buf: Vec::new() })?;
                page.attach_orphaned(&bundle_key)?;
                let key = page.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = page.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut pages[cache_index];
                cache_line.replace(page.as_mut_owned());

                // Write the entry to the inode and return its reference
                bundle.as_mut().pages.insert(index as usize, Some(key));
                dio.commit().await?;
                key
            }
        };

        // Use the cache-line to load the page
        let cache_index = page.as_u64() as usize % CACHED_BUNDLES;
        let cache_line = &mut pages[cache_index];
        let page = match cache_line {
            Some(b) if *b.key() == page => {
                // Cache-hit
                b
            }
            _ => {
                // Cache-miss - load the page into the cacheline and return its pages
                let dao = dio.load::<Page>(&page).await?;
                let dao = dao.as_mut_owned();
                cache_line.replace(dao);
                cache_line.as_mut().unwrap()
            }
        };

        // Expand the buffer until it has bytes at the spot we are writing
        while page.buf.len() < offset as usize {
            page.buf.push(0);
        }

        // Write to the page
        let mut writer = Cursor::new(&mut page.buf);
        writer.set_position(offset);
        tokio::io::copy(reader, &mut writer).await?;
        Ok(())
    }

    pub async fn commit(&mut self) -> Result<()> {
        let (dirty, inode, _, pages) = match self {
            FileState::Mutable {
                dirty,
                inode,
                bundles,
                pages,
            } => (dirty, inode, bundles, pages),
            FileState::Immutable {
                inode: _,
                bundles: _,
                pages: _,
            } => {
                return Ok(());
            }
        };

        if *dirty {
            for page in pages.iter_mut() {
                page.take();
            }
            let dio = inode.trans();
            dio.commit().await?;
            *dirty = false;
        }
        Ok(())
    }

    pub async fn set_xattr(&mut self, name: &str, value: &str) -> Result<()> {
        match self {
            FileState::Mutable {
                dirty: _,
                inode,
                bundles: _,
                pages: _,
            } => {
                inode
                    .as_mut()
                    .xattr
                    .insert(name.to_string(), value.to_string())
                    .await?
            }
            FileState::Immutable {
                inode: _,
                bundles: _,
                pages: _,
            } => {
                bail!(FileSystemErrorKind::NoAccess);
            }
        };
        Ok(())
    }

    pub async fn remove_xattr(&mut self, name: &str) -> Result<bool> {
        let name = name.to_string();
        let ret = match self {
            FileState::Mutable {
                dirty: _,
                inode,
                bundles: _,
                pages: _,
            } => inode.as_mut().xattr.delete(&name).await?,
            FileState::Immutable {
                inode: _,
                bundles: _,
                pages: _,
            } => {
                bail!(FileSystemErrorKind::NoAccess);
            }
        };
        Ok(ret)
    }

    pub async fn get_xattr(&self, name: &str) -> Result<Option<String>> {
        let name = name.to_string();
        let ret = match self {
            FileState::Mutable {
                dirty: _,
                inode,
                bundles: _,
                pages: _,
            } => inode.xattr.get(&name).await?.map(|a| a.deref().clone()),
            FileState::Immutable {
                inode,
                bundles: _,
                pages: _,
            } => inode.xattr.get(&name).await?.map(|a| a.deref().clone()),
        };
        Ok(ret)
    }

    pub async fn list_xattr(&self) -> Result<FxHashMap<String, String>> {
        let mut ret = FxHashMap::default();
        match self {
            FileState::Mutable {
                dirty: _,
                inode,
                bundles: _,
                pages: _,
            } => {
                for (k, v) in inode.xattr.iter().await? {
                    ret.insert(k, v.deref().clone());
                }
            }
            FileState::Immutable {
                inode,
                bundles: _,
                pages: _,
            } => {
                for (k, v) in inode.xattr.iter().await? {
                    ret.insert(k, v.deref().clone());
                }
            }
        }
        Ok(ret)
    }
}

#[async_trait]
impl FileApi for RegularFile {
    fn kind(&self) -> FileKind {
        FileKind::RegularFile
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
        self.size.read()
    }

    fn mode(&self) -> u32 {
        self.mode
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn created(&self) -> u64 {
        self.created
    }

    fn updated(&self) -> u64 {
        self.updated
    }

    fn accessed(&self) -> u64 {
        self.updated
    }

    async fn fallocate(&self, size: u64) -> Result<()> {
        let mut lock = self.state.lock().await;
        lock.set_size(size)?;
        *self.size.lock_write() = size;
        Ok(())
    }

    async fn read(&self, mut offset: u64, mut size: u64) -> Result<Bytes> {
        // Clip the read to the correct size (or return EOF)
        let mut state = self.state.lock().await;
        let file_size = state.get_size()?;
        if offset >= file_size {
            //return Err(libc::EOF.into());
            return Ok(Bytes::from(Vec::new()));
        }
        size = size.min(file_size - offset);
        if size <= 0 {
            return Ok(Bytes::from(Vec::new()));
        }

        // Prepare
        let stride_page = super::model::PAGE_SIZE as u64;
        let mut ret = Vec::with_capacity(size as usize);
        let mut cursor = Cursor::new(&mut ret);

        // Read the data (under a lock)
        while size > 0 {
            let sub_offset = offset % stride_page;
            let sub_size = size.min(stride_page - sub_offset);
            if sub_size > 0 {
                state.read_page(offset, sub_size, &mut cursor).await?;
            }

            // Move the data pointers and offsets
            size = size - sub_size;
            offset = offset + sub_size;
        }

        Ok(Bytes::from(ret))
    }

    async fn write(&self, mut offset: u64, data: &[u8]) -> Result<u64> {
        // Validate
        let mut size = data.len();
        if size <= 0 {
            return Ok(0);
        }

        // Prepare
        let stride_page = super::model::PAGE_SIZE;

        // Update the inode
        let mut state = self.state.lock().await;
        let end = offset + data.len() as u64;
        if end > state.get_size()? {
            state.set_size(end)?;
        }

        // Write the data (under a lock)
        let mut data_offset = 0 as usize;
        while size > 0 {
            let sub_offset = offset % stride_page as u64;
            let sub_size = size.min(stride_page - sub_offset as usize);
            if sub_size > 0 {
                let mut reader = Cursor::new(&data[data_offset..(data_offset + sub_size) as usize]);
                state.write_page(offset, &mut reader).await?;
            }

            // Move the data pointers and offsets
            size = size - sub_size;
            offset = offset + sub_size as u64;
            data_offset = data_offset + sub_size as usize;
        }

        // Update the size of the file if it has grown in side (do this update lock to prevent race conditions)
        *self.size.lock_write() = state.get_size()?;

        // Success
        Ok(data.len() as u64)
    }

    async fn commit(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.commit().await?;
        Ok(())
    }

    async fn set_xattr(&mut self, name: &str, value: &str) -> Result<()> {
        let mut state = self.state.lock().await;
        state.set_xattr(name, value).await
    }

    async fn remove_xattr(&mut self, name: &str) -> Result<bool> {
        let mut state = self.state.lock().await;
        state.remove_xattr(name).await
    }

    async fn get_xattr(&self, name: &str) -> Result<Option<String>> {
        let state = self.state.lock().await;
        state.get_xattr(name).await
    }

    async fn list_xattr(&self) -> Result<FxHashMap<String, String>> {
        let state = self.state.lock().await;
        state.list_xattr().await
    }
}
