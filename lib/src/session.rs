#[allow(unused_imports)]
use serde::{Serialize, Deserialize, de::DeserializeOwned};

use crate::{crypto::*};
use crate::spec::MessageFormat;
use crate::conf::ConfAte;

#[derive(Debug, Serialize, Deserialize, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RolePurpose
{
    Owner,
    Delegate,
    Contributor,
    Observer,
    Other(String),
}

impl std::fmt::Display
for RolePurpose
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RolePurpose::Owner => write!(f, "owner"),
            RolePurpose::Delegate => write!(f, "delegate"),
            RolePurpose::Contributor => write!(f, "contributor"),
            RolePurpose::Observer => write!(f, "observer"),
            RolePurpose::Other(a) => write!(f, "other-{}", a),
        }
    }
}

impl std::str::FromStr
for RolePurpose
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(RolePurpose::Owner),
            "delegate" => Ok(RolePurpose::Delegate),
            "contributor" => Ok(RolePurpose::Contributor),
            "observer" => Ok(RolePurpose::Observer),
            a if a.starts_with("other-") && a.len() > 6 => Ok(RolePurpose::Other(a["other-".len()..].to_string())),
            _ => Err("valid values are 'owner', 'delegate', 'contributor', 'observer' and 'other-'"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupRole
{
    pub purpose: RolePurpose,
    pub properties: Vec<SessionProperty>,
}

impl GroupRole
{
    pub fn add_read_key(&mut self, key: &EncryptKey) {
        self.properties.push(SessionProperty::ReadKey(key.clone()));
    }

    pub fn add_private_read_key(&mut self, key: &PrivateEncryptKey) {
        self.properties.push(SessionProperty::PrivateReadKey(key.clone()));
    }

    pub fn add_write_key(&mut self, key: &PrivateSignKey) {
        self.properties.push(SessionProperty::WriteKey(key.clone()));
    }

    pub fn add_identity(&mut self, identity: String) {
        self.properties.push(SessionProperty::Identity(identity));
    }

    pub fn read_keys<'a>(&'a self) -> impl Iterator<Item = &'a EncryptKey> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    SessionProperty::ReadKey(k) => Some(k),
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
                    SessionProperty::WriteKey(k) => Some(k),
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
                    SessionProperty::PublicReadKey(k) => Some(k),
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
                    SessionProperty::PrivateReadKey(k) => Some(k),
                    _ => None
                }
            )
    }

    pub fn identity<'a>(&'a self) -> Option<&'a String> {
        self.properties
            .iter()
            .filter_map(
                |p| match p
                {
                    SessionProperty::Identity(k) => Some(k),
                    _ => None
                }
            )
            .next()
    }
}

impl std::fmt::Display
for GroupRole
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Group
{
    pub name: String,
    pub roles: Vec<GroupRole>,
}

impl Group
{
    pub fn get_role<'a>(&'a self, purpose: &RolePurpose) -> Option<&'a GroupRole>
    {
        self.roles.iter().filter(|r| r.purpose == *purpose).next()
    }

    pub fn get_or_create_role<'a>(&'a mut self, purpose: &RolePurpose) -> &'a mut GroupRole
    {
        if self.roles.iter().any(|r| r.purpose == *purpose) == false {
            self.roles.push(GroupRole {
                purpose: purpose.clone(),
                properties: Vec::new()
            });
        }

        self.roles.iter_mut().filter(|r| r.purpose == *purpose).next().expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
    }
    
    pub fn add_read_key(&mut self, purpose: &RolePurpose, key: &EncryptKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(SessionProperty::ReadKey(key.clone()));
    }

    pub fn add_private_read_key(&mut self, purpose: &RolePurpose, key: &PrivateEncryptKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(SessionProperty::PrivateReadKey(key.clone()));
    }

    pub fn add_write_key(&mut self, purpose: &RolePurpose, key: &PrivateSignKey) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(SessionProperty::WriteKey(key.clone()));
    }

    pub fn add_identity(&mut self, purpose: &RolePurpose, identity: String) {
        let role = self.get_or_create_role(purpose);
        role.properties.push(SessionProperty::Identity(identity));
    }

    pub fn read_keys<'a>(&'a self) -> impl Iterator<Item = &'a EncryptKey> {
        self.roles
            .iter()
            .flat_map(|r| r.properties.iter())
            .filter_map(
                |p| match p
                {
                    SessionProperty::ReadKey(k) => Some(k),
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
                    SessionProperty::WriteKey(k) => Some(k),
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
                    SessionProperty::PublicReadKey(k) => Some(k),
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
                    SessionProperty::PrivateReadKey(k) => Some(k),
                    _ => None
                }
            )
    }
}

impl std::fmt::Display
for Group
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

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SessionProperty
{
    None,
    ReadKey(EncryptKey),
    PrivateReadKey(PrivateEncryptKey),
    PublicReadKey(PublicEncryptKey),
    WriteKey(PrivateSignKey),
    Identity(String),
}

impl Default for SessionProperty {
    fn default() -> Self {
        SessionProperty::None
    }
}

impl std::fmt::Display
for SessionProperty
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionProperty::None => write!(f, "none"),
            SessionProperty::ReadKey(a) => write!(f, "read-key:{}", a),
            SessionProperty::PrivateReadKey(a) => write!(f, "private-read-key:{}", a),
            SessionProperty::PublicReadKey(a) => write!(f, "public-read-key:{}", a),
            SessionProperty::WriteKey(a) => write!(f, "write-key:{}", a),
            SessionProperty::Identity(a) => write!(f, "identity:{}", a),
        }
    }
}

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
pub struct Session
where Self: Send + Sync
{
    pub log_format: Option<MessageFormat>,
    pub user: GroupRole,
    pub sudo: Option<GroupRole>,
    pub groups: Vec<Group>,
}

impl Default
for Session
{
    fn default() -> Session {
        Session {
            user: GroupRole {
                purpose: RolePurpose::Delegate,
                properties: Vec::new()
            },
            sudo: None,
            groups: Vec::new(),
            log_format: None
        }
    }
}

impl Session
{
    pub fn new(cfg: &ConfAte) -> Session {
        let mut ret = Session::default();
        ret.log_format = Some(cfg.log_format);
        ret
    }

    pub fn get_group_role<'a>(&'a self, group: &String, purpose: &RolePurpose) -> Option<&'a GroupRole>
    {
        self.groups.iter().filter(|r| r.name == *group).next()
            .map(|a| a.get_role(purpose))
            .flatten()
    }

    pub fn get_or_create_group_role<'a>(&'a mut self, group: &String, purpose: &RolePurpose) -> &'a mut GroupRole
    {
        if self.groups.iter().any(|r| r.name == *group) == false {
            self.groups.push(Group {
                name: group.clone(),
                roles: Vec::new()
            });
        }

        self.groups.iter_mut().filter(|r| r.name == *group).next()
            .expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
            .get_or_create_role(purpose)
    }

    pub fn get_group<'a>(&'a mut self, group: &String) -> Option<&'a mut Group>
    {
        self.groups.iter_mut().filter(|r| r.name == *group).next()
    }

    pub fn get_or_create_group<'a>(&'a mut self, group: &String) -> &'a mut Group
    {
        if self.groups.iter().any(|r| r.name == *group) == false {
            self.groups.push(Group {
                name: group.clone(),
                roles: Vec::new()
            });
        }

        self.groups.iter_mut().filter(|r| r.name == *group).next()
            .expect("It should not be possible for this call to fail as the line above just added the item we are searching for")
    }

    pub fn get_or_create_sudo<'a>(&'a mut self) -> &'a mut GroupRole
    {
        if self.sudo.is_none() {
            self.sudo.replace(GroupRole {
                purpose: RolePurpose::Owner,
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

    pub fn add_group_read_key(&mut self, group: &String, purpose: &RolePurpose, key: &EncryptKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_read_key(key)
    }

    pub fn add_group_private_read_key(&mut self, group: &String, purpose: &RolePurpose, key: &PrivateEncryptKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_private_read_key(key)
    }

    pub fn add_group_write_key(&mut self, group: &String, purpose: &RolePurpose, key: &PrivateSignKey) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_write_key(key)
    }

    pub fn add_group_identity(&mut self, group: &String, purpose: &RolePurpose, identity: String) {
        let role = self.get_or_create_group_role(group, purpose);
        role.add_identity(identity)
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

    pub fn append(&mut self, mut other: Session) {

        if self.log_format.is_none() {
            self.log_format = other.log_format;
        }

        self.user.properties.append(&mut other.user.properties);

        if let Some(mut sudo) = other.sudo {
            let b = self.sudo.get_or_insert(GroupRole {
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
for Session
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[user=")?;
        self.user.fmt(f)?;
        for group in self.groups.iter() {
            write!(f, ",")?;
            group.fmt(f)?;
        }
        write!(f, "]")
    }
}