#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{crypto::*};

use super::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AteGroup
{
    pub name: String,
    pub roles: Vec<AteGroupRole>,
    pub broker_read: Option<PrivateEncryptKey>,
    pub broker_write: Option<PrivateSignKey>,
}

impl AteGroup
{
    pub fn get_role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole>
    {
        self.roles.iter().filter(|r| r.purpose == *purpose).next()
    }

    pub fn get_or_create_role<'a>(&'a mut self, purpose: &AteRolePurpose) -> &'a mut AteGroupRole
    {
        if self.roles.iter().any(|r| r.purpose == *purpose) == false {
            self.roles.push(AteGroupRole {
                purpose: purpose.clone(),
                properties: Vec::new()
            });
        }

        self.roles.iter_mut().filter(|r| r.purpose == *purpose).next().expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
    }
    
    pub fn add_read_key(&mut self, purpose: &AteRolePurpose, key: &EncryptKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(AteSessionProperty::ReadKey(key.clone()));
    }

    pub fn add_private_read_key(&mut self, purpose: &AteRolePurpose, key: &PrivateEncryptKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(AteSessionProperty::PrivateReadKey(key.clone()));
    }

    pub fn add_write_key(&mut self, purpose: &AteRolePurpose, key: &PrivateSignKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(AteSessionProperty::WriteKey(key.clone()));
    }

    pub fn add_identity(&mut self, purpose: &AteRolePurpose, identity: String) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(AteSessionProperty::Identity(identity));
    }

    pub fn read_keys<'a>(&'a self) -> impl Iterator<Item = &'a EncryptKey> {
        self.roles
            .iter()
            .flat_map(|r| r.properties.iter())
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::ReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn write_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateSignKey> {
        self.roles
            .iter()
            .flat_map(|r| r.properties.iter())
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::WriteKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn public_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PublicEncryptKey> {
        self.roles
            .iter()
            .flat_map(|r| r.properties.iter())
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::PublicReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn private_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateEncryptKey> {
        self.roles
            .iter()
            .flat_map(|r| r.properties.iter())
            .filter_map(
                |p| match p
                {
                    AteSessionProperty::PrivateReadKey(k) => Some(k),
                    _ => None
                }
            )
    }
}

impl std::fmt::Display
for AteGroup
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(name={}", self.name)?;
        for role in self.roles.iter() {
            write!(f, ",")?;
            role.fmt(f)?;
        }
        write!(f, ")")
    }
}