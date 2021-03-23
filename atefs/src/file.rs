use std::ops::DerefMut;

use async_trait::async_trait;
use crate::api::FileApi;
use serde::*;
use fuse3::FileType;
use super::model::*;
use ate::prelude::PrimaryKey;
use super::api::SpecType;
use ate::prelude::*;
use bytes::Bytes;
use fuse3::{Errno, Result};
use super::fs::conv_load;
use super::fs::conv_serialization;
use tokio::sync::Mutex;
use parking_lot::Mutex as PMutex;

#[derive(Debug)]
pub struct RegularFile
{
    pub inode: Mutex<Dao<Inode>>,
    pub ino: u64,
    pub created: u64,
    pub updated: u64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub name: String,
    pub size: PMutex<u64>,
}

impl RegularFile
{
    pub fn new(inode: Dao<Inode>, created: u64, updated: u64) -> RegularFile {
        RegularFile {
            size: PMutex::new(inode.size),
            uid: inode.dentry.uid,
            gid: inode.dentry.gid,
            mode: inode.dentry.mode,
            name: inode.dentry.name.clone(),
            ino: inode.key().as_u64(),
            inode: Mutex::new(inode),
            created,
            updated,
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
        *self.size.lock()
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

    async fn fallocate(&self, size: u64)
    {
        *self.size.lock() = size;
        self.inode.lock().await.size = size;
    }

    async fn read(&self, chain: &Chain, session: &AteSession, offset: u64, size: u32) -> Result<Bytes>
    {
        read_data(&self.inode, chain, session, offset, size).await
    }

    async fn write(&self, chain: &Chain, session: &AteSession, offset: u64, data: &[u8]) -> Result<u64>    
    {
        write_data(&self.inode, chain, session, offset, data).await
    }
}

pub(crate) async fn read_data(inode: &Mutex<Dao<Inode>>, chain: &Chain, session: &AteSession, offset: u64, size: u32) -> Result<Bytes>
{
    let mut size = size as u64;
    let mut offset = offset;
    let mut ret = Vec::new();

    // Load the bundles into memory
    let mut dio;
    let bundles = {
        let mut lock = inode.lock().await;
        dio = chain.dio_for_dao(session, TransactionScope::None, lock.deref_mut()).await;

        // Clip the size to within the bounds of the file and do some early exits
        if offset >= lock.size {
            return Ok(Bytes::from(ret))
        }
        if size > lock.size - offset {
            size = lock.size - offset;
        }
        if size <= 0 {
            return Ok(Bytes::from(ret))
        }

        lock.bundles.clone()
    };

    // Walk the bundles
    let stride_page = super::model::PAGE_SIZE as u64;
    let stride_bundle = super::model::BUNDLE_SIZE as u64 * stride_page;
    for bundle in bundles.into_iter()
    {
        // If we are done
        if size <= 0 { break; }

        // If the bundle is miles to the left then ignore it
        if offset >= stride_bundle {
            offset = offset - stride_bundle;
            continue;
        }
        
        // Load the bundle (if its a hole then skip it)
        let bundle = match bundle {
            Some(a) => conv_load(dio.load::<PageBundle>(&a).await)?,
            None =>
            {
                // Write a bunch of zeros that represent this hole
                let next = size.min(stride_bundle - offset);
                for _ in 0..next {
                    ret.push(0);
                }

                // Update the offset and size then keep going
                offset = 0;
                size = size - next;
                continue;
            }
        };

        // Load bundle from the chain-of-trust and iterate through the pages
        for page in bundle.pages.iter()
        {
            // If we are done
            if size <= 0 { break; }
            
            // If the page is entirely to the left then ignore it
            if offset >= stride_page {
                offset = offset - stride_page;
                continue;
            }

            // Clip the read bytes to the page size and lead the page (if its a hole then skip it)
            let next = size.min(stride_page - offset);
            let page = match page {
                Some(k) => conv_load(dio.load::<Page>(&k).await)?,
                None => {
                    for _ in 0..next {
                        ret.push(0);
                    }

                    // Update the offset and size then keep going
                    offset = 0;
                    size = size - next;
                    continue;
                }
            };
                    
            // It might be a partial page
            let sub_next = next.min(page.buf.len() as u64 - offset);
            if sub_next > 0 {
                use std::io::Cursor;
                let ret_end = ret.len() as u64;
                let mut writer = Cursor::new(&mut ret);
                writer.set_position(ret_end);
                let mut reader = Cursor::new(&page.buf[offset as usize..(offset + sub_next) as usize]);
                tokio::io::copy(&mut reader, &mut writer).await?;
            }

            // Finish the last bit with zeros
            for _ in sub_next..next {
                ret.push(0);
            }
            
            // Update the position and move onto the next page
            offset = 0;
            size = size - next;
        }
    }

    // Anything that is left just add it as zeros
    for _ in 0..size {
        ret.push(0);
    }

    // Return the result
    Ok(Bytes::from(ret))
}

pub(crate) async fn write_data(inode: &Mutex<Dao<Inode>>, chain: &Chain, session: &AteSession, offset: u64, data: &[u8]) -> Result<u64>
{
    let size = data.len() as u64;
    let mut offset = offset;
    let mut ret = 0 as u64;

    // Lock the object and get a DIO
    let mut lock = inode.lock().await;
    let mut dio = chain.dio_for_dao(session, TransactionScope::None, lock.deref_mut()).await;

    // Update the size of the file (if it expands)
    let new_file_size = lock.size.max(offset + data.len() as u64);
    if new_file_size > lock.size {
        lock.size = new_file_size;
    }

    // Add missing bundles up to the range we need
    let range = offset + size;
    let stride_page = super::model::PAGE_SIZE as u64;
    let stride_bundle = super::model::BUNDLE_SIZE as u64 * stride_page;
    while (lock.bundles.len() as u64 * stride_bundle) < range {
        lock.bundles.push(None);
    }

    // Now its time to write to all the bundles that are impacted
    let mut remaining = size;
    for bundle in lock.bundles.iter_mut() {
        if remaining <= 0 { break; }

        // If the bundle is miles to the left then ignore it
        if offset >= stride_bundle {
            offset = offset - stride_bundle;
            continue;
        }

        // Get or create the bundle
        let mut bundle = match bundle.as_mut() {
            Some(a) => conv_load(dio.load::<PageBundle>(a).await)?,
            None => {
                let b = conv_serialization(dio.store(PageBundle {
                    pages: Vec::new()
                }))?;
                bundle.replace(b.key().clone());
                b
            }
        };

        // Add all the pages for this bundle
        while bundle.pages.len() < super::model::BUNDLE_SIZE {
            bundle.pages.push(None);
        }

        // Loop through all the pages
        for page in bundle.pages.iter_mut() {
            if remaining <= 0 { break; }

            // If the page is entirely to the left then ignore it
            if offset >= stride_page {
                offset = offset - stride_page;
                continue;
            }

            // Get or create the page
            let mut page = match page.as_mut() {
                Some(a) => conv_load(dio.load::<Page>(a).await)?,
                None => {
                    let p = conv_serialization(dio.store(Page {
                        buf: Vec::new()
                    }))?;
                    page.replace(p.key().clone());
                    p
                }
            };

            // Write zeros up to the offset
            while (page.buf.len() as u64) < offset {
                page.buf.push(0);
            }

            // Build the cursors
            use std::io::Cursor;
            let next = remaining.min(stride_page - offset);
            let ret_next = ret + next;

            // Write the data to the buffer
            let mut writer = Cursor::new(&mut page.buf);
            writer.set_position(offset);
            let mut reader = Cursor::new(&data[ret as usize..ret_next as usize]);
            tokio::io::copy(&mut reader, &mut writer).await?;
            ret = ret + next;
            
            // Clear the offset
            offset = 0;
            remaining = remaining - next;
        }
    }

    // Commit the main object manually (as the DIO has a shorter scope lifetime)
    conv_serialization(lock.commit())?;
    
    // Return the result
    Ok(ret)
}