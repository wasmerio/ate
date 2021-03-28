#![allow(dead_code)]
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
use super::fs::conv_commit;
use super::fs::conv_serialization;
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

#[derive(Debug)]
pub struct RegularFile
{
    pub ino: u64,
    pub created: u64,
    pub updated: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub name: String,
    pub scope: TransactionScope,
    pub size: SeqLock<u64>,
    pub state: Mutex<FileState>,
}

#[derive(Debug)]
pub struct FileState
{
    pub inode: Dao<Inode>,
    pub dirty: bool,
    pub pages: [Option<Dao<Page>>; CACHED_PAGES],
    pub bundles: [Option<Dao<PageBundle>>; CACHED_BUNDLES],
}

impl FileState
{
    pub fn get_size(&mut self) -> Result<u64>
    {
        Ok(self.inode.size)
    }

    pub fn set_size(&mut self, val: u64) -> Result<()>
    {
        self.inode.size = val;
        self.dirty = true;
        Ok(())
    }

    pub async fn read_page(&mut self, chain: &Chain, session: &AteSession, mut offset: u64, mut size: u64, ret: &mut Cursor<&mut Vec<u8>>) -> Result<()>
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
                let mut dio = chain.dio_ext(session, TransactionScope::None).await;                    
                let dao = conv_load(dio.load::<PageBundle>(&bundle).await)?;
                
                // We always commit changes to the bundles so no need to commit it here
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
                let mut dio = chain.dio_ext(session, TransactionScope::None).await;
                let dao = conv_load(dio.load::<Page>(&page).await)?;
                
                // Replace the cache-line - pages here are lazy-written so we might need to flush it
                let old = cache_line.replace(dao);
                if let Some(mut old) = old {
                    conv_serialization(old.commit(&mut dio))?;
                }
                conv_commit(dio.commit().await)?;
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

    pub async fn write_page(&mut self, chain: &Chain, session: &AteSession, mut offset: u64, reader: &mut Cursor<&[u8]>, scope: TransactionScope) -> Result<()>
    {
        self.dirty = true;

        // Compute the strides
        let stride_page = super::model::PAGE_SIZE as u64;
        let stride_bundle = super::model::PAGES_PER_BUNDLE as u64 * stride_page;

        // Expand the bundles until we have enough of them to cover this write offset
        let index = offset / stride_bundle;
        while self.inode.bundles.len() <= index as usize {
            self.inode.bundles.push(None);
        }
        offset = offset - (index * stride_bundle);

        // If the bundle is a hole then we need to fill it
        let bundle_ref = &mut self.inode.bundles[index as usize];
        let bundle = match bundle_ref {
            Some(a) => a.clone(),
            None => {
                // Create the bundle
                let bundle = Dao::make(PrimaryKey::generate(), chain.default_format(), 
                PageBundle {
                        pages: Vec::new(),
                    }
                );
                let key = bundle.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = bundle.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut self.bundles[cache_index];
                if let Some(mut old) = cache_line.replace(bundle) {
                    if old.is_dirty() {
                        let mut dio = chain.dio_ext(session, scope).await;
                        conv_serialization(old.commit(&mut dio))?;
                        conv_commit(dio.commit().await)?;
                    }
                }

                // Write the entry to the inode and return its reference
                bundle_ref.replace(key);
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
                let mut dio = chain.dio_ext(session, scope).await;                    
                let dao = conv_load(dio.load::<PageBundle>(&bundle).await)?;
                
                // We always commit changes to the bundles so no need to commit it here
                cache_line.replace(dao);
                cache_line.as_mut().unwrap()
            }
        };

        // Expand the page until we have enough of them to cover this write offset
        let index = offset / stride_page;
        while bundle.pages.len() <= index as usize {
            bundle.pages.push(None);
        }
        offset = offset - (index * stride_page);

        // If the page is a hole then we need to fill it
        let page_ref = &mut bundle.pages[index as usize];
        let page = match page_ref {
            Some(a) => a.clone(),
            None => {
                // Create the page (and commit it for reference integrity)
                let mut dio = chain.dio_ext(session, scope).await;
                let page = conv_serialization(dio.store(Page {
                        buf: Vec::new(),
                    }
                ))?;
                let key = page.key().clone();

                // Replace the cache-line with this new one (if something was left behind then commit it)
                // (in the next section we will commit the row out of this match statement)
                let cache_index = page.key().as_u64() as usize % CACHED_BUNDLES;
                let cache_line = &mut self.pages[cache_index];
                if let Some(mut old) = cache_line.replace(page) {
                    if old.is_dirty() {
                        conv_serialization(old.commit(&mut dio))?;
                    }
                }

                // Write the entry to the inode and return its reference
                conv_commit(dio.commit().await)?;
                page_ref.replace(key);
                key
            }
        };

        // Now its time to commit all the metadata updates (if there were any)
        if bundle.is_dirty() || self.inode.is_dirty() {
            let mut dio = chain.dio_ext(session, scope).await;
            if self.inode.is_dirty() {
                conv_serialization(bundle.commit(&mut dio))?;    
            }
            if self.inode.is_dirty() {
                conv_serialization(self.inode.commit(&mut dio))?;
            }
            conv_commit(dio.commit().await)?
        }

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
                let mut dio = chain.dio_ext(session, scope).await;                    
                let dao = conv_load(dio.load::<Page>(&page).await)?;
                
                // We always commit changes to the bundles so no need to commit it here
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

    pub async fn commit(&mut self, chain: &Chain, session: &AteSession) -> Result<()>
    {
        if self.dirty {
            let mut dio = chain.dio_ext(session, TransactionScope::None).await;
            for page in self.pages.iter_mut() {
                if let Some(page) = page {
                    conv_serialization(page.commit(&mut dio))?;
                }
            }
            conv_commit(dio.commit().await)?;
            self.dirty = false;
        }
        Ok(())
    }
}

impl RegularFile
{
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> RegularFile {
        RegularFile {
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            size: SeqLock::new(inode.size),
            created,
            updated,
            scope: TransactionScope::None,
            state: Mutex::new(FileState {
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

    async fn fallocate(&self, _chain: &Chain, _session: &AteSession, size: u64) -> Result<()>
    {
        let mut lock = self.state.lock().await;
        lock.set_size(size)?;
        *self.size.lock_write() = size;
        Ok(())
    }

    async fn read(&self, chain: &Chain, session: &AteSession, mut offset: u64, mut size: u64) -> Result<Bytes>
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
                state.read_page(chain, session, offset, sub_size, &mut cursor).await?;
            }

            // Move the data pointers and offsets
            size = size - sub_size;
            offset = offset + sub_size;
        }
        
        Ok(Bytes::from(ret))
    }

    async fn write(&self, chain: &Chain, session: &AteSession, mut offset: u64, data: &[u8]) -> Result<u64>    
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
            state.inode.size = end;
        }
        
        // Write the data (under a lock)
        let mut data_offset = 0 as usize;
        while size > 0 {
            let sub_offset = offset % stride_page as u64;
            let sub_size = size.min(stride_page - sub_offset as usize);
            if sub_size > 0 {
                let mut reader = Cursor::new(&data[data_offset..(data_offset+sub_size) as usize]);
                state.write_page(chain, session, offset, &mut reader, self.scope).await?;
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

    async fn commit(&self, chain: &Chain, session: &AteSession) -> Result<()> {
        let mut state = self.state.lock().await;
        state.commit(chain, session).await?;
        Ok(())
    }
}