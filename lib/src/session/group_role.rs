#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{crypto::*};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AteGroupRole
{
    pub purpose: AteRolePurpose,
    pub properties: Vec<AteSessionProperty>,
}

impl AteGroupRole
{
    pub fn add_read_key(&mut self, key: &EncryptKey) {
        self.properties.push(AteSessionProperty::ReadKey(key.clone()));
    }

    pub fn add_private_read_key(&mut self, key: &PrivateEncryptKey) {
        self.properties.push(AteSessionProperty::PrivateReadKey(key.clone()));
    }

    pub fn add_write_key(&mut self, key: &PrivateSignKey) {
        self.properties.push(AteSessionProperty::WriteKey(key.clone()));
    }

    pub fn clear_read_keys(&mut self) {
        self.properties.retain(|p| {
            if let AteSessionProperty::ReadKey(_) = p {
                return false;
            }
            return true;
        });
    }

    pub fn clear_private_read_keys(&mut self) {
        self.properties.retain(|p| {
            if let AteSessionProperty::PrivateReadKey(_) = p {
                return false;
            }
            return true;
        });
    }

    pub fn clear_write_keys(&mut self) {
        self.properties.retain(|p| {
            if let AteSessionProperty::WriteKey(_) = p {
                return false;
            }
            return true;
        });
    }

    pub fn add_uid(&mut self, uid: u32) {
        self.properties.push(AteSessionProperty::Uid(uid));
    }

    pub fn add_gid(&mut self, gid: u32) {
        self.properties.push(AteSessionProperty::Gid(gid));
    }

    pub fn read_keys<'a>(&'a self) -> impl Iterator<Item = &'a EncryptKey> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::ReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn write_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateSignKey> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::WriteKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn public_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PublicEncryptKey> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::PublicReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn private_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateEncryptKey> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::PrivateReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn uid<'a>(&'a self) -> Option<u32> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::Uid(k) => Some(k.clone()),
                    _ => None
                }
            )
            .next()
    }

    pub fn gid<'a>(&'a self) -> Option<u32> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::Gid(k) => Some(k.clone()),
                    _ => None
                }
            )
            .next()
    }
}

impl std::fmt::Display
for AteGroupRole
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(purpose={}", self.purpose)?;
        for prop in self.properties.iter() {
            write!(f, ",")?;
            prop.fmt(f)?;
        }
        write!(f, ")")
    }
}