#![allow(dead_code)]
#![allow(unused_imports)]
use log::{info, error, debug};
use std::sync::Arc;
use std::ops::DerefMut;
use async_trait::async_trait;
use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;
use ate::prelude::PrimaryKey;
use super::api::SpecType;
use ate::prelude::*;
use bytes::{Bytes, BytesMut, Buf, BufMut};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use fuse3::{Errno, Result};
use super::fs::conv;
use super::fs::conv_load;
use super::fs::cc;
use super::fs::cs;
use tokio::sync::Mutex;
use parking_lot::{Mutex as PMutex, RwLock};
use fxhash::FxHashMap;
use cached::Cached;
use std::ops::Deref;
use seqlock::SeqLock;
use xarc::{AtomicXarc, Xarc};
use lockfree::prelude::*;
use core::sync::atomic::{AtomicPtr, Ordering};
use std::io::Cursor;

const CACHED_BUNDLES: usize = 10;      // Number of cached bundles per open file
const CACHED_PAGES: usize = 80;       // Number of cached pages per open file
const ZERO_PAGE: [u8; super::model::PAGE_SIZE] = [0 as u8; super::model::PAGE_SIZE];    // Page full of zeros

pub struct RegularFile
{
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

pub struct FileState
{
    pub dirty: bool,
    pub dio: Arc<DioMut>,
    pub inode: DaoMut<Inode>,
    pub bundles: [Option<DaoMut<PageBundle>>; CACHED_BUNDLES],
    pub pages: [Option<DaoMutGuardOwned<Page>>; CACHED_PAGES],
    
}

impl FileState
{
    pub fn get_size(&mut self) -> Result<u64>
    {
        Ok(self.inode.size)
    }

    pub fn set_size(&mut self, val: u64) -> Result<()>
    {
        self.inode.as_mut().size = val;
        self.dirty = true;
        Ok(())
    }

    pub async fn read_page(&mut self, mut offset: u64, mut size: u64, ret: &mut Cursor<&mut Vec<u8>>) -> Result<()>
    {
        // Compute the strides
        let stride_page = super::model::PAGE_SIZE as u64;
        let stride_bundle = super::model::PAGES_PER_BUNDLE as u64 * stride_page;

        // First we index into the right bundle
        let index = offset / stride_bundle;
        if index >= self.inode.bundles.len() as u64 {
            FileState::write_zeros(ret, size).await?;
            return Ok(());
        }
        let bundle = self.inode.bundles[index as usize];
        offset = offset - (index * stride_bundle);

        // If its a hole then just write zeros
        let bundle = match bundle {
            Some(a) => a,
            None => {
                FileState::write_zeros(ret, size).await?;
                return Ok(());
            }
        };

        // Use the cache-line to load the bundle
        let cache_index = bundle.as_u64() as usize % CACHED_BUNDLES;
        let cache_line = &mut self.bundles[cache_index];
        let bundle = match cache_line {
            Some(b)
                if *b.key() == bundle => {
                // Cache-hit
                b
            },
            _ => {
                // Cache-miss - load the bundle into the cacheline and return its pages
                let dao = conv_load(self.dio.load::<PageBundle>(&bundle).await)?;
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
        let cache_line = &mut self.pages[cache_index];
        let page = match cache_line {
            Some(b)
            if *b.key() == page => {
                // Cache-hit
                b
            },
            _ => {
                // Cache-miss - load the bundle into the cacheline and return its pages
                let dao = conv_load(self.dio.load::<Page>(&page).await)?;
                cache_line.replace(dao.as_mut_owned());
                cache_line.as_mut().unwrap()
            }
        };

        // Read the bytes from the page
        let buf = &page.buf;
        let sub_next = size.min(buf.len() as u64 - offset);
        if sub_next > 0 {
            let mut reader = Cursor::new(&buf[offset as usize..(offset + sub_next) as usize]);
            tokio::io::copy(&mut reader, ret).await?;
            size = size - sub_next;
        }

        // Finish off the last bit with zeros and return the result
        FileState::write_zeros(ret, size).await?;
        Ok(())
    }

    pub async fn write_zeros(ret: &mut Cursor<&mut Vec<u8>>, size: u64) -> Result<()>
    {
        if size <= 0 {
            return Ok(())
        }

        let offset = super::model::PAGE_SIZE as u64 - size;
        let mut reader = Cursor::new(&ZERO_PAGE);
        reader.set_position(offset);
        
        tokio::io::copy(&mut reader, ret).await?;
        Ok(())
    }

    pub async fn write_page(&mut self, mut offset: u64, reader: &mut Cursor<&[u8]>) -> Result<()>
    {
        self.dirty = true;

        // Compute the strides
        let inode_key = self.inode.key().clone();
        let stride_page = super::model::PAGE_SIZE as u64;
        let stride_bundle = super::model::PAGES_PER_BUNDLE as u64 * stride_page;

        // Expand the bundles until we have enough of them to cover this write offset
        let index = offset / stride_bundle;
        if self.inode.bundles.len() <= index as usize {
            let mut guard = self.inode.as_mut();
            while guard.bundles.len() <= index as usize {
                guard.bundles.push(None);
            }
            drop(guard);
            cc(self.dio.commit().await)?;
        }
        offset = offset - (index * stride_bundle);

        // If the bundle is a hole then we need to fill it
        let bundle = match self.inode.bundles[index as usize] {
            Some(a) => a,
            None => {
                // Create the bundle
                let mut bundle = cs(self.dio.store( 
                    PageBundle {
                            pages: Vec::new(),
                        }))?;
                cs(bundle.attach_orphaned(&inode_key))?;
                
                let key = bundle.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = bundle.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut self.bundles[cache_index];
                cache_line.replace(bundle);

                // Write the entry to the inode and return its reference
                self.inode.as_mut().bundles.insert(index as usize, Some(key));
                cc(self.dio.commit().await)?;
                key
            }
        };

        // Use the cache-line to load the bundle
        let cache_index = bundle.as_u64() as usize % CACHED_BUNDLES;
        let cache_line = &mut self.bundles[cache_index];
        let bundle = match cache_line {
            Some(b)
                if *b.key() == bundle => {
                // Cache-hit
                b
            },
            _ => {
                // Cache-miss - load the bundle into the cacheline and return its pages
                let dao = conv_load(self.dio.load::<PageBundle>(&bundle).await)?;
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
            cc(self.dio.commit().await)?;
        }
        offset = offset - (index * stride_page);

        // If the page is a hole then we need to fill it
        let bundle_key = bundle.key().clone();
        let page_ref = bundle.pages[index as usize];
        let page = match page_ref {
            Some(a) => a.clone(),
            None => {
                // Create the page (and commit it for reference integrity)
                let mut page = cs(self.dio.store(Page {
                        buf: Vec::new(),
                    },
                ))?;
                cs(page.attach_orphaned(&bundle_key))?;
                let key = page.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = page.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut self.pages[cache_index];
                cache_line.replace(page.as_mut_owned());

                // Write the entry to the inode and return its reference
                bundle.as_mut().pages.insert(index as usize, Some(key));
                cc(self.dio.commit().await)?;
                key
            }
        };

        // Use the cache-line to load the page
        let cache_index = page.as_u64() as usize % CACHED_BUNDLES;
        let cache_line = &mut self.pages[cache_index];
        let page = match cache_line {
            Some(b)
                if *b.key() == page => {
                // Cache-hit
                b
            },
            _ => {
                // Cache-miss - load the page into the cacheline and return its pages
                let dao = conv_load(self.dio.load::<Page>(&page).await)?;
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

    pub async fn commit(&mut self) -> Result<()>
    {
        if self.dirty {
            for page in self.pages.iter_mut() {
                page.take();
            }
            cc(self.dio.commit().await)?;
            self.dirty = false;
        }
        Ok(())
    }
}

impl RegularFile
{
    pub async fn new(inode: Dao<Inode>, created: u64, updated: u64, scope: TransactionScope) -> RegularFile {
        let dio = inode.dio().trans(scope).await;
        let inode = inode.as_mut(&dio);
        RegularFile {
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            size: SeqLock::new(inode.size),
            created,
            updated,
            state: Mutex::new(FileState {
                dio,
                inode,
                dirty: false,
                bundles: array_init::array_init(|_| None),
                pages: array_init::array_init(|_| None),
            }),
        }
    }
}

#[async_trait]
impl FileApi
for RegularFile
{
    fn spec(&self) -> SpecType {
        SpecType::RegularFile
    }

    fn ino(&self) -> u64 {
        self.ino
    }

    fn kind(&self) -> FileType {
        FileType::RegularFile
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

    async fn fallocate(&self, size: u64) -> Result<()>
    {
        let mut lock = self.state.lock().await;
        lock.set_size(size)?;
        *self.size.lock_write() = size;
        Ok(())
    }

    async fn read(&self, mut offset: u64, mut size: u64) -> Result<Bytes>
    {
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

    async fn write(&self, mut offset: u64, data: &[u8]) -> Result<u64>    
    {
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
        if end > state.inode.size {
            state.inode.as_mut().size = end;
            cc(state.inode.trans().commit().await)?;
        }
        
        // Write the data (under a lock)
        let mut data_offset = 0 as usize;
        while size > 0 {
            let sub_offset = offset % stride_page as u64;
            let sub_size = size.min(stride_page - sub_offset as usize);
            if sub_size > 0 {
                let mut reader = Cursor::new(&data[data_offset..(data_offset+sub_size) as usize]);
                state.write_page(offset, &mut reader).await?;
            }

            // Move the data pointers and offsets
            size = size - sub_size;
            offset = offset + sub_size as u64;
            data_offset = data_offset + sub_size as usize;
        }

        // Update the size of the file if it has grown in side (do this update lock to prevent race conditions)
        *self.size.lock_write() = state.inode.size;

        // Success
        Ok(data.len() as u64)
    }

    async fn commit(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        state.commit().await?;
        Ok(())
    }
}