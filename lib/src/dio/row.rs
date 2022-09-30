#![allow(unused_imports)]
use error_chain::bail;
use fxhash::FxHashSet;
use tracing::{debug, warn, error};

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard};

use crate::crypto::{EncryptedPrivateKey, PrivateSignKey};
use crate::{crypto::EncryptKey, session::AteSessionProperty};

use super::dio_mut::*;
use crate::crypto::AteHash;
use crate::dio::*;
use crate::error::*;
use crate::event::*;
use crate::header::*;
use crate::index::*;
use crate::meta::*;
use crate::spec::*;

pub use super::vec::DaoVec;

#[derive(Debug, Clone)]
pub(crate) struct RowHeader {
    pub key: PrimaryKey,
    pub parent: Option<MetaParent>,
    pub auth: MetaAuthorization,
}

pub(super) struct Row<D> {
    pub(super) key: PrimaryKey,
    pub(super) type_name: String,
    pub(super) created: u64,
    pub(super) updated: u64,
    pub(super) format: MessageFormat,
    pub(super) data: D,
    pub(super) collections: FxHashSet<MetaCollection>,
    pub(super) extra_meta: Vec<CoreMetadata>,
    pub(super) is_new: bool,
}

impl<D> Clone for Row<D>
where
    D: Clone,
{
    fn clone(&self) -> Self {
        Row {
            key: self.key.clone(),
            type_name: self.type_name.clone(),
            created: self.created,
            updated: self.updated,
            format: self.format,
            data: self.data.clone(),
            collections: self.collections.clone(),
            extra_meta: self.extra_meta.clone(),
            is_new: self.is_new.clone(),
        }
    }
}

impl<D> std::fmt::Debug for Row<D>
where
    D: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "row(key={}, created={}, updated={}, data=",
            self.key, self.created, self.updated
        )?;
        let ret = self.data.fmt(f);
        write!(f, ")")?;
        ret
    }
}

impl<D> Row<D> {
    pub(crate) fn from_event(
        dio: &Arc<Dio>,
        evt: &EventStrongData,
        created: u64,
        updated: u64,
    ) -> Result<(RowHeader, Row<D>), SerializationError>
    where
        D: DeserializeOwned,
    {
        let key = match evt.meta.get_data_key() {
            Some(key) => key,
            None => {
                bail!(SerializationErrorKind::NoPrimarykey)
            }
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
                let parent = match evt.meta.get_parent() {
                    Some(a) => Some(a.clone()),
                    None => None,
                };

                let data = {
                    let _pop1 = DioScope::new(dio);
                    let _pop2 = PrimaryKeyScope::new(key);

                    evt.format.data.deserialize_ref(&data)
                        .map_err(SerializationError::from)
                        .map_err(|err| {
                            //trace!("{}", String::from_utf8_lossy(&data[..]));
                            err
                        })?
                };

                Ok((
                    RowHeader {
                        key: key.clone(),
                        parent,
                        auth,
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
                        is_new: false,
                    },
                ))
            },
            None => bail!(SerializationErrorKind::NoData),
        }
    }

    pub(crate) fn from_row_data(
        dio: &Arc<Dio>,
        row: &RowData,
    ) -> Result<(RowHeader, Row<D>), SerializationError>
    where
        D: DeserializeOwned,
    {
        let data = {
            let _pop1 = DioScope::new(dio);
            let _pop2 = PrimaryKeyScope::new(row.key);

            row.format.data.deserialize_ref(&row.data)
                .map_err(SerializationError::from)?
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
                is_new: false,
            },
        ))
    }

    pub(crate) fn as_row_data(
        &self,
        header: &RowHeader,
    ) -> std::result::Result<RowData, SerializationError>
    where
        D: Serialize,
    {
        let data = Bytes::from(self.format.data.serialize(&self.data)?);
        let data_hash = AteHash::from_bytes(&data[..]);
        Ok(RowData {
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
            is_new: self.is_new,
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowData {
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
    pub is_new: bool,
}
