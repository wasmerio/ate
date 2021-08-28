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
pub struct AteSessionGroup
{
    pub inner: AteSessionInner,
    pub group: AteGroup,
}

impl AteSessionGroup
{
    pub fn new(inner: AteSessionInner, group: String) -> AteSessionGroup {
        AteSessionGroup {
            inner,
            group: AteGroup {
                name: group,
                roles: Vec::new(),
                broker_read: None,
                broker_write: None,
            }
        }
    }

    pub fn get_group_role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole>
    {
        self.group.get_role(purpose)
    }

    pub fn get_or_create_group_role<'a>(&'a mut self, purpose: &AteRolePurpose) -> &'a mut AteGroupRole
    {
        self.group.get_or_create_role(purpose)
    }

    pub fn add_group_read_key(&mut self, purpose: &AteRolePurpose, key: &EncryptKey) {
        let role = self.get_or_create_group_role(purpose);
        role.add_read_key(key)
    }

    pub fn add_group_private_read_key(&mut self, purpose: &AteRolePurpose, key: &PrivateEncryptKey) {
        let role = self.get_or_create_group_role(purpose);
        role.add_private_read_key(key)
    }

    pub fn add_group_write_key(&mut self, purpose: &AteRolePurpose, key: &PrivateSignKey) {
        let role = self.get_or_create_group_role(purpose);
        role.add_write_key(key)
    }

    pub fn add_group_gid(&mut self, purpose: &AteRolePurpose, gid: u32) {
        let role = self.get_or_create_group_role(purpose);
        role.add_gid(gid)
    }
}

impl AteSession
for AteSessionGroup
{
    fn role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        self.get_group_role(purpose)
    }

    fn read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.group.roles.iter().flat_map(|a| a.read_keys()));
        }
        let ret1 = self.inner.read_keys(category);
        let ret2 = self.group.roles.iter()
            .filter(move |_| category.includes_group_keys())
            .flat_map(|a| a.read_keys());
        Box::new(ret1.chain(ret2))
    }

    fn write_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.group.roles.iter().flat_map(|a| a.write_keys()));
        }
        let ret1 = self.inner.write_keys(category);
        let ret2 = self.group.roles.iter()
            .filter(move |_| category.includes_group_keys())
            .flat_map(|a| a.write_keys());
        Box::new(ret1.chain(ret2))
    }

    fn public_read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.group.roles.iter().flat_map(|a| a.public_read_keys()));
        }
        let ret1 = self.inner.public_read_keys(category);
        let ret2 = self.group.roles.iter()
            .filter(move |_| category.includes_group_keys())
            .flat_map(|a| a.public_read_keys());
        Box::new(ret1.chain(ret2))
    }

    fn private_read_keys<'a>(&'a self, category: AteSessionKeyCategory) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        if category == AteSessionKeyCategory::UpperKeys {
            return Box::new(self.group.roles.iter().flat_map(|a| a.private_read_keys()));
        }
        let ret1 = self.inner.private_read_keys(category);
        let ret2 = self.group.roles.iter()
            .filter(move |_| category.includes_group_keys())
            .flat_map(|a| a.private_read_keys());
        Box::new(ret1.chain(ret2))
    }

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey> {
        self.group.broker_read.as_ref()
    }
    
    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey> {
        self.group.broker_write.as_ref()
    }

    fn identity<'a>(&'a self) -> &'a str {
        self.group.name.as_str()
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
        if let Some(gid) = self.group.roles
            .iter()
            .flat_map(|a| a.gid())
            .next() {
            return Some(gid)
        }
        self.inner.gid()
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn clone_inner(&self) -> AteSessionInner {
        self.inner.clone()
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        let ret1 = self.inner.properties();
        let ret2 = self.group.roles.iter()
            .flat_map(|a| a.properties.iter());
        Box::new(ret1.chain(ret2))
    }

    fn append<'a, 'b>(&'a mut self, properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>) {
        self.inner.append(properties);
    }
}

impl Deref
for AteSessionGroup
{
    type Target = AteSessionInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut
for AteSessionGroup
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl std::fmt::Display
for AteSessionGroup
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[inner=")?;
        self.inner.fmt(f)?;
        write!(f, ",group=")?;
        self.group.fmt(f)?;
        write!(f, "]")
    }
}