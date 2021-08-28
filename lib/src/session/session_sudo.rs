#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::ops::Deref;
use std::ops::DerefMut;

use crate::{crypto::*};

use super::*;

/// Sudo sessions are elevated permissions used to carry out
/// high priveledge actions
///
/// Sessions are never cached and only exist in memory for the
/// duration that you use them for security reasons.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AteSessionSudo
{
    pub inner: AteSessionUser,
    pub sudo: AteGroupRole,
}

impl Default
for AteSessionSudo
{
    fn default() -> AteSessionSudo {
        AteSessionSudo {
            inner: AteSessionUser::default(),
            sudo: AteGroupRole {
                purpose: AteRolePurpose::Owner,
                properties: Vec::new()
            },
        }
    }
}

impl AteSessionSudo
{
    pub fn new() -> AteSessionSudo {
        AteSessionSudo::default()
    }

    pub fn add_sudo_read_key(&mut self, key: &EncryptKey) {
        self.sudo.add_read_key(key)
    }

    pub fn add_sudo_private_read_key(&mut self, key: &PrivateEncryptKey) {
        self.sudo.add_private_read_key(key)
    }

    pub fn add_sudo_write_key(&mut self, key: &PrivateSignKey) {
        self.sudo.add_write_key(key)
    }
}

impl AteSession
for AteSessionSudo
{
    fn role<'a>(&'a self, _purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        None
    }

    fn read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.sudo.read_keys());
        }
        let ret1 = self.inner.read_keys(category);
        let ret2 = self.sudo.read_keys()
            .filter(move |_| category.includes_sudo_keys());
        Box::new(ret1.chain(ret2))
    }

    fn write_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.sudo.write_keys());
        }
        let ret1 = self.inner.write_keys(category);
        let ret2 = self.sudo.write_keys()
            .filter(move |_| category.includes_sudo_keys());
        Box::new(ret1.chain(ret2))
    }

    fn public_read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.sudo.public_read_keys());
        }
        let ret1 = self.inner.public_read_keys(category);
        let ret2 = self.sudo.public_read_keys()
            .filter(move |_| category.includes_sudo_keys());
        Box::new(ret1.chain(ret2))
    }

    fn private_read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.sudo.private_read_keys());
        }
        let ret1 = self.inner.private_read_keys(category);
        let ret2 = self.sudo.private_read_keys()
            .filter(move |_| category.includes_sudo_keys());
        Box::new(ret1.chain(ret2))
    }

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey> {
        self.inner.broker_read()
    }
    
    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey> {
        self.inner.broker_write()
    }

    fn identity<'a>(&'a self) -> &'a str {
        self.inner.identity.as_str()
    }

    fn user<'a>(&'a self) -> &'a AteSessionUser {
        self.inner.user()
    }

    fn user_mut<'a>(&'a mut self) -> &'a mut AteSessionUser {
        self.inner.user_mut()
    }

    fn uid<'a>(&'a self) -> Option<u32> {
        self.inner.uid()
    }

    fn gid<'a>(&'a self) -> Option<u32> {
        self.inner.gid()
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn clone_inner(&self) -> AteSessionInner {
        AteSessionInner::Sudo(self.clone())
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        let ret1 = self.inner.properties();
        let ret2 = self.sudo.properties.iter();
        Box::new(ret1.chain(ret2))
    }

    fn append<'a, 'b>(&'a mut self, properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>) {
        self.inner.append(properties);
    }
}

impl Deref
for AteSessionSudo
{
    type Target = AteSessionUser;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut
for AteSessionSudo
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl std::fmt::Display
for AteSessionSudo
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[user=")?;
        self.inner.user.fmt(f)?;
        write!(f, ",sudo=")?;
        self.sudo.fmt(f)?;
        write!(f, "]")
    }
}