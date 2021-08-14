#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{crypto::*};
use crate::spec::MessageFormat;
use crate::conf::ConfAte;

use super::*;

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
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AteSession
{
    pub log_format: Option<MessageFormat>,
    pub user: AteGroupRole,
    pub sudo: Option<AteGroupRole>,
    pub groups: Vec<AteGroup>,
    pub broker_read: Option<PrivateEncryptKey>,
    pub broker_write: Option<PrivateSignKey>,
}

impl Default
for AteSession
{
    fn default() -> AteSession {
        AteSession {
            user: AteGroupRole {
                purpose: AteRolePurpose::Delegate,
                properties: Vec::new()
            },
            sudo: None,
            groups: Vec::new(),
            log_format: None,
            broker_read: None,
            broker_write: None
        }
    }
}

impl AteSession
{
    pub fn new(cfg: &ConfAte) -> AteSession {
        let mut ret = AteSession::default();
        ret.log_format = Some(cfg.log_format);
        ret
    }

    pub fn get_group_role<'a>(&'a self, group: &String, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole>
    {
        self.groups.iter().filter(|r| r.name == *group).next()
            .map(|a| a.get_role(purpose))
            .flatten()
    }

    pub fn get_or_create_group_role<'a>(&'a mut self, group: &String, purpose: &AteRolePurpose) -> &'a mut AteGroupRole
    {
        if self.groups.iter().any(|r| r.name == *group) == false {
            self.groups.push(AteGroup {
                name: group.clone(),
                roles: Vec::new(),
                broker_read: None,
                broker_write: None,
            });
        }

        self.groups.iter_mut().filter(|r| r.name == *group).next()
            .expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
            .get_or_create_role(purpose)
    }

    pub fn get_group<'a>(&'a self, group: &String) -> Option<&'a AteGroup>
    {
        self.groups.iter().filter(|r| r.name == *group).next()
    }

    pub fn get_or_create_group<'a>(&'a mut self, group: &String) -> &'a mut AteGroup
    {
        if self.groups.iter().any(|r| r.name == *group) == false {
            self.groups.push(AteGroup {
                name: group.clone(),
                roles: Vec::new(),
                broker_read: None,
                broker_write: None,
            });
        }

        self.groups.iter_mut().filter(|r| r.name == *group).next()
            .expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
    }

    pub fn get_or_create_sudo<'a>(&'a mut self) -> &'a mut AteGroupRole
    {
        if self.sudo.is_none() {
            self.sudo.replace(AteGroupRole {
                purpose: AteRolePurpose::Owner,
                properties: Vec::new()
            });
        }

        self.sudo.iter_mut().next()
            .expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
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

    pub fn add_user_identity(&mut self, identity: String) {
        self.user.add_identity(identity)
    }

    pub fn add_user_uid(&mut self, uid: u32) {
        self.user.add_uid(uid)
    }

    pub fn add_sudo_read_key(&mut self, key: &EncryptKey) {
        self.get_or_create_sudo().add_read_key(key)
    }

    pub fn add_sudo_private_read_key(&mut self, key: &PrivateEncryptKey) {
        self.get_or_create_sudo().add_private_read_key(key)
    }

    pub fn add_sudo_write_key(&mut self, key: &PrivateSignKey) {
        self.get_or_create_sudo().add_write_key(key)
    }

    pub fn add_sudo_identity(&mut self, identity: String) {
        self.get_or_create_sudo().add_identity(identity)
    }

    pub fn add_group_read_key(&mut self, group: &String, purpose: &AteRolePurpose, key: &EncryptKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_read_key(key)
    }

    pub fn add_group_private_read_key(&mut self, group: &String, purpose: &AteRolePurpose, key: &PrivateEncryptKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_private_read_key(key)
    }

    pub fn add_group_write_key(&mut self, group: &String, purpose: &AteRolePurpose, key: &PrivateSignKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_write_key(key)
    }

    pub fn add_group_identity(&mut self, group: &String, purpose: &AteRolePurpose, identity: String) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_identity(identity)
    }

    pub fn add_group_gid(&mut self, group: &String, purpose: &AteRolePurpose, gid: u32) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_gid(gid)
    }

    pub fn read_keys<'a>(&'a self) -> impl Iterator<Item = &'a EncryptKey> {
        let ret1 = self.user.read_keys();
        let ret2 = self.sudo.iter().flat_map(|a| a.read_keys());
        let ret3 = self.groups
            .iter()
            .flat_map(|g| g.roles.iter())
            .flat_map(|a| a.read_keys());
        ret1.chain(ret2).chain(ret3)
    }

    pub fn write_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateSignKey> {
        let ret1 = self.user.write_keys();
        let ret2 = self.sudo.iter().flat_map(|a| a.write_keys());
        let ret3 = self.groups
            .iter()
            .flat_map(|g| g.roles.iter())
            .flat_map(|a| a.write_keys());
        ret1.chain(ret2).chain(ret3)
    }

    pub fn public_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PublicEncryptKey> {
        let ret1 = self.user.public_read_keys();
        let ret2 = self.sudo.iter().flat_map(|a| a.public_read_keys());
        let ret3 = self.groups
            .iter()
            .flat_map(|g| g.roles.iter())
            .flat_map(|a| a.public_read_keys());
        ret1.chain(ret2).chain(ret3)
    }

    pub fn private_read_keys<'a>(&'a self) -> impl Iterator<Item = &'a PrivateEncryptKey> {
        let ret1 = self.user.private_read_keys();
        let ret2 = self.sudo.iter().flat_map(|a| a.private_read_keys());
        let ret3 = self.groups
            .iter()
            .flat_map(|g| g.roles.iter())
            .flat_map(|a| a.private_read_keys());
        ret1.chain(ret2).chain(ret3)
    }

    pub fn append(&mut self, mut other: AteSession) {

        if self.log_format.is_none() {
            self.log_format = other.log_format;
        }

        self.user.properties.append(&mut other.user.properties);

        if let Some(mut sudo) = other.sudo {
            let b = self.sudo.get_or_insert(AteGroupRole {
                purpose: sudo.purpose,
                properties: Vec::new()
            });
            b.properties.append(&mut sudo.properties);
        }

        for group in other.groups {
            self.get_or_create_group(&group.name);
            for mut role in group.roles {
                let b = self.get_or_create_group_role(&group.name, &role.purpose);
                b.properties.append(&mut role.properties);
            }
        }
    }
}

impl std::fmt::Display
for AteSession
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[user=")?;
        self.user.fmt(f)?;
        if let Some(sudo) = &self.sudo {
            write!(f, ",sudo=")?;
            sudo.fmt(f)?;
        }
        for group in self.groups.iter() {
            write!(f, ",")?;
            group.fmt(f)?;
        }
        write!(f, "]")
    }
}