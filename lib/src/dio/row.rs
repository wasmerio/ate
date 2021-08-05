#![allow(unused_imports)]
use tracing::{warn, debug};
use fxhash::FxHashSet;

use serde::{Serialize, de::DeserializeOwned};
use bytes::Bytes;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use parking_lot::{Mutex, MutexGuard};

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::{AteSession, AteSessionProperty}};

use super::dio_mut::*;
use crate::header::*;
use crate::event::*;
use crate::meta::*;
use crate::error::*;
use crate::crypto::AteHash;
use crate::dio::*;
use crate::spec::*;
use crate::index::*;

pub use super::vec::DaoVec;

#[derive(Debug, Clone)]
pub(crate) struct RowHeader
{
    pub key: PrimaryKey,
    pub parent: Option<MetaParent>,
    pub auth: MetaAuthorization,
}

pub(super) struct Row<D>
{
    pub(super) key: PrimaryKey,
    pub(super) type_name: String,
    pub(super) created: u64,
    pub(super) updated: u64,
    pub(super) format: MessageFormat,
    pub(super) data: D,
    pub(super) collections: FxHashSet<MetaCollection>,
    pub(super) extra_meta: Vec<CoreMetadata>,
}

impl<D> Clone
for Row<D>
where D: Clone,
{
    fn clone(&self) -> Self
    {
        Row {
            key: self.key.clone(),
            type_name: self.type_name.clone(),
            created: self.created,
            updated: self.updated,
            format: self.format,
            data: self.data.clone(),
            collections: self.collections.clone(),
            extra_meta: self.extra_meta.clone(),
        }
    }
}

impl<D> std::fmt::Debug
for Row<D>
where D: std::fmt::Debug
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "row(key={}, created={}, updated={}, data=", self.key, self.created, self.updated)?;
        let ret = self.data.fmt(f);
        write!(f, ")")?;
        ret
    }
}

impl<D> Row<D>
{
    pub(crate) fn from_event(dio: &Arc<Dio>, evt: &EventData, created: u64, updated: u64) -> Result<(RowHeader, Row<D>), SerializationError>
    where D: DeserializeOwned,
    {
        let key = match evt.meta.get_data_key() {
            Some(key) => key,
            None => { return Result::Err(SerializationError::NoPrimarykey) }
        };
        let mut collections = FxHashSet::default();
        for a in evt.meta.get_collections() {
            collections.insert(a);
        }
        match &evt.data_bytes {
            Some(data) => {
                let auth = match evt.meta.get_authorization() {
                    Some(a) => a.clone(),
                    None => MetaAuthorization::default(),
                };
                let parent = match evt.meta.get_parent() { Some(a) => Some(a.clone()), None => None };

                let data = {
                    let _pop1 = DioScope::new(dio);
                    let _pop2 = PrimaryKeyScope::new(key);

                    evt.format.data.deserialize(&data)?
                };

                Ok((
                    RowHeader {
                        key: key.clone(),
                        parent,
                        auth
                    },
                    Row {
                        key,
                        type_name: std::any::type_name::<D>().to_string(),
                        format: evt.format,
                        data,
                        collections,
                        created,
                        updated,
                        extra_meta: Vec::new(),
                    }
                ))
            }
            None => return Result::Err(SerializationError::NoData),
        }
    }

    pub(crate) fn from_row_data(dio: &Arc<Dio>, row: &RowData) -> Result<(RowHeader, Row<D>), SerializationError>
    where D: DeserializeOwned,
    {
        let data = {
            let _pop1 = DioScope::new(dio);
            let _pop2 = PrimaryKeyScope::new(row.key);

            row.format.data.deserialize(&row.data)?
        };

        Ok((
            RowHeader {
                key: row.key.clone(),
                parent: row.parent.clone(),
                auth: row.auth.clone(),
            },
            Row {
                key: row.key,
                type_name: row.type_name.clone(),
                format: row.format,
                data: data,
                collections: row.collections.clone(),
                created: row.created,
                updated: row.updated,
                extra_meta: row.extra_meta.clone(),
            }
        ))
    }

    pub(crate) fn as_row_data(&self, header: &RowHeader) -> std::result::Result<RowData, SerializationError>
    where D: Serialize,
    {
        let data = Bytes::from(self.format.data.serialize(&self.data)?);            
        let data_hash = AteHash::from_bytes(&data[..]);
        Ok
        (
            RowData {
                key: self.key.clone(),
                type_name: self.type_name.clone(),
                format: self.format,
                parent: header.parent.clone(),
                data_hash,
                data,
                auth: header.auth.clone(),
                collections: self.collections.clone(),
                created: self.created,
                updated: self.updated,
                extra_meta: self.extra_meta.clone(),
            }
        )
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData
{
    pub key: PrimaryKey,
    pub type_name: String,
    pub format: MessageFormat,
    pub data_hash: AteHash,
    pub data: Bytes,
    pub collections: FxHashSet<MetaCollection>,
    pub created: u64,
    pub updated: u64,
    pub extra_meta: Vec<CoreMetadata>,
    pub parent: Option<MetaParent>,
    pub auth: MetaAuthorization,
}