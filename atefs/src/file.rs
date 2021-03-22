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
        let mut size = size as u64;
        let mut offset = offset;
        let mut ret = Vec::new();

        // Load the bundles into memory
        let mut dio;
        let bundles = {
            let lock = self.inode.lock().await;
            dio = chain.dio_for_dao(session, TransactionScope::None, &*lock).await;

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
            // If the bundle is miles to the left then ignore it
            if offset >= stride_bundle {
                offset = offset - stride_bundle;
                continue;
            }
            
            // The bundle could be a hole
            match bundle {

                // The bundle might have some data
                Some(bundle) =>
                {
                    // Load bundle from the chain-of-trust and iterate through the pages
                    let bundle = conv_load(dio.load::<PageBundle>(&bundle).await)?;
                    for page in bundle.pages.iter()
                    {
                        // If the page is entirely to the left then ignore it
                        if offset >= stride_page {
                            offset = offset - stride_page;
                            continue;
                        }

                        // Clip the read bytes to the page size
                        let next = size.min(stride_page - offset);
                        match page
                        {
                            // The page is lead so load it
                            Some(k) => {
                                let page = conv_load(dio.load::<Page>(&k).await)?;
                                
                                // It might be a partial page
                                let sub_next = next.min(page.buf.len() as u64 - offset);
                                if sub_next > 0 {
                                    ret.copy_from_slice(&page.buf[offset as usize..(offset + sub_next) as usize]);
                                }

                                // Finish the last bit with zeros
                                for _ in sub_next..next {
                                    ret.push(0);
                                }
                            },

                            // The page is whole so just write zeros for it all
                            None => {
                                for _ in 0..next {
                                    ret.push(0);
                                }
                            }
                        }
                        
                        // Update the position and move onto the next page
                        offset = 0;
                        size = size - next;
                        continue;
                    }
                },

                // The bundle is a hole
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
            }
        }

        // Anything that is left just add it as zeros
        for _ in 0..size {
            ret.push(0);
        }

        // Return the result
        Ok(Bytes::from(ret))
    }
}