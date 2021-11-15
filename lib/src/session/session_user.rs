#[allow(unused_imports)]
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::crypto::*;

use super::*;

pub type SessionToken = Option<EncryptedSecureData<EncryptKey>>;

/// Sessions hold facts about the user that give them certains
/// rights and abilities to view data within the chain-of-trust.
///
/// For instance - to see encrypted data for specific users they
/// must insert their `EncryptKey` into this session before
/// accessing the chain via a `Dio`.
///
/// Another example is the ability to write data. For certain
/// records within the tree if they have been protected with
/// write protection then you must insert your `PrivateKey`
/// into the session before you attempt to insert or modify these
/// records.
///
/// Sessions are never cached and only exist in memory for the
/// duration that you use them for security reasons.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AteSessionUser {
    pub user: AteGroupRole,
    pub token: SessionToken,
    pub identity: String,
    pub broker_read: Option<PrivateEncryptKey>,
    pub broker_write: Option<PrivateSignKey>,
}

impl Default for AteSessionUser {
    fn default() -> AteSessionUser {
        AteSessionUser {
            user: AteGroupRole {
                purpose: AteRolePurpose::Personal,
                properties: Vec::new(),
            },
            token: None,
            identity: "nobody@nowhere.com".to_string(),
            broker_read: None,
            broker_write: None,
        }
    }
}

impl AteSessionUser {
    pub fn new() -> AteSessionUser {
        AteSessionUser::default()
    }

    pub fn add_user_read_key(&mut self, key: &EncryptKey) {
        self.user.add_read_key(key)
    }

    pub fn add_user_private_read_key(&mut self, key: &PrivateEncryptKey) {
        self.user.add_private_read_key(key)
    }

    pub fn add_user_write_key(&mut self, key: &PrivateSignKey) {
        self.user.add_write_key(key)
    }

    pub fn add_user_uid(&mut self, uid: u32) {
        self.user.add_uid(uid)
    }
}

impl AteSession for AteSessionUser {
    fn role<'a>(&'a self, _purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        None
    }

    fn read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.user.read_keys());
        }
        let ret1 = self
            .user
            .read_keys()
            .filter(move |_| category.includes_user_keys());
        Box::new(ret1)
    }

    fn write_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.user.write_keys());
        }
        let ret1 = self
            .user
            .write_keys()
            .filter(move |_| category.includes_user_keys());
        Box::new(ret1)
    }

    fn public_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.user.public_read_keys());
        }
        let ret1 = self
            .user
            .public_read_keys()
            .filter(move |_| category.includes_user_keys());
        Box::new(ret1)
    }

    fn private_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.user.private_read_keys());
        }
        let ret1 = self
            .user
            .private_read_keys()
            .filter(move |_| category.includes_user_keys());
        Box::new(ret1)
    }

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey> {
        self.broker_read.as_ref()
    }

    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey> {
        self.broker_write.as_ref()
    }

    fn identity<'a>(&'a self) -> &'a str {
        self.identity.as_str()
    }

    fn user<'a>(&'a self) -> &'a AteSessionUser {
        self
    }

    fn user_mut<'a>(&'a mut self) -> &'a mut AteSessionUser {
        self
    }

    fn uid<'a>(&'a self) -> Option<u32> {
        self.user.uid()
    }

    fn gid<'a>(&'a self) -> Option<u32> {
        if let Some(gid) = self.user.gid() {
            Some(gid)
        } else {
            self.uid()
        }
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        let ret1 = self.user.properties.iter();
        Box::new(ret1)
    }

    fn append<'a, 'b>(
        &'a mut self,
        properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>,
    ) {
        let mut properties = properties.map(|a| a.clone()).collect::<Vec<_>>();
        self.user.properties.append(&mut properties);
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn clone_inner(&self) -> AteSessionInner {
        AteSessionInner::User(self.clone())
    }
}

impl std::fmt::Display for AteSessionUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        self.user.fmt(f)?;
        write!(f, "]")
    }
}
