use super::*;
use crate::crypto::*;
use serde::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AteSessionType {
    User(AteSessionUser),
    Sudo(AteSessionSudo),
    Group(AteSessionGroup),
}

impl AteSession for AteSessionType {
    fn role<'a>(&'a self, purpose: &AteRolePurpose) -> Option<&'a AteGroupRole> {
        match self {
            AteSessionType::User(a) => a.role(purpose),
            AteSessionType::Sudo(a) => a.role(purpose),
            AteSessionType::Group(a) => a.role(purpose),
        }
    }

    fn read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a EncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.read_keys(category),
            AteSessionType::Sudo(a) => a.read_keys(category),
            AteSessionType::Group(a) => a.read_keys(category),
        }
    }

    fn write_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateSignKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.write_keys(category),
            AteSessionType::Sudo(a) => a.write_keys(category),
            AteSessionType::Group(a) => a.write_keys(category),
        }
    }

    fn public_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PublicEncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.public_read_keys(category),
            AteSessionType::Sudo(a) => a.public_read_keys(category),
            AteSessionType::Group(a) => a.public_read_keys(category),
        }
    }

    fn private_read_keys<'a>(
        &'a self,
        category: AteSessionKeyCategory,
    ) -> Box<dyn Iterator<Item = &'a PrivateEncryptKey> + 'a> {
        match self {
            AteSessionType::User(a) => a.private_read_keys(category),
            AteSessionType::Sudo(a) => a.private_read_keys(category),
            AteSessionType::Group(a) => a.private_read_keys(category),
        }
    }

    fn broker_read<'a>(&'a self) -> Option<&'a PrivateEncryptKey> {
        match self {
            AteSessionType::User(a) => a.broker_read(),
            AteSessionType::Sudo(a) => a.broker_read(),
            AteSessionType::Group(a) => a.broker_read(),
        }
    }

    fn broker_write<'a>(&'a self) -> Option<&'a PrivateSignKey> {
        match self {
            AteSessionType::User(a) => a.broker_write(),
            AteSessionType::Sudo(a) => a.broker_write(),
            AteSessionType::Group(a) => a.broker_write(),
        }
    }

    fn identity<'a>(&'a self) -> &'a str {
        match self {
            AteSessionType::User(a) => a.identity(),
            AteSessionType::Sudo(a) => a.identity(),
            AteSessionType::Group(a) => a.identity(),
        }
    }

    fn user<'a>(&'a self) -> &'a AteSessionUser {
        match self {
            AteSessionType::User(a) => a.user(),
            AteSessionType::Sudo(a) => a.user(),
            AteSessionType::Group(a) => a.user(),
        }
    }

    fn user_mut<'a>(&'a mut self) -> &'a mut AteSessionUser {
        match self {
            AteSessionType::User(a) => a.user_mut(),
            AteSessionType::Sudo(a) => a.user_mut(),
            AteSessionType::Group(a) => a.user_mut(),
        }
    }

    fn uid<'a>(&'a self) -> Option<u32> {
        match self {
            AteSessionType::User(a) => a.uid(),
            AteSessionType::Sudo(a) => a.uid(),
            AteSessionType::Group(a) => a.uid(),
        }
    }

    fn gid<'a>(&'a self) -> Option<u32> {
        match self {
            AteSessionType::User(a) => a.gid(),
            AteSessionType::Sudo(a) => a.gid(),
            AteSessionType::Group(a) => a.gid(),
        }
    }

    fn clone_session(&self) -> Box<dyn AteSession> {
        Box::new(self.clone())
    }

    fn clone_inner(&self) -> AteSessionInner {
        match self {
            AteSessionType::User(a) => a.clone_inner(),
            AteSessionType::Sudo(a) => a.clone_inner(),
            AteSessionType::Group(a) => a.clone_inner(),
        }
    }

    fn properties<'a>(&'a self) -> Box<dyn Iterator<Item = &'a AteSessionProperty> + 'a> {
        match self {
            AteSessionType::User(a) => a.properties(),
            AteSessionType::Sudo(a) => a.properties(),
            AteSessionType::Group(a) => a.properties(),
        }
    }

    fn append<'a, 'b>(
        &'a mut self,
        properties: Box<dyn Iterator<Item = &'b AteSessionProperty> + 'b>,
    ) {
        match self {
            AteSessionType::User(a) => a.append(properties),
            AteSessionType::Sudo(a) => a.append(properties),
            AteSessionType::Group(a) => a.append(properties),
        }
    }
}

impl std::fmt::Display for AteSessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        match self {
            AteSessionType::User(a) => write!(f, "user: {}", a),
            AteSessionType::Sudo(a) => write!(f, "sudo: {}", a),
            AteSessionType::Group(a) => write!(f, "group: {}", a),
        }?;
        write!(f, "]")
    }
}

impl From<AteSessionInner> for AteSessionType {
    fn from(a: AteSessionInner) -> Self {
        match a {
            AteSessionInner::User(a) => AteSessionType::User(a),
            AteSessionInner::Sudo(a) => AteSessionType::Sudo(a),
        }
    }
}

impl From<AteSessionUser> for AteSessionType {
    fn from(a: AteSessionUser) -> Self {
        AteSessionType::User(a)
    }
}

impl From<AteSessionSudo> for AteSessionType {
    fn from(a: AteSessionSudo) -> Self {
        AteSessionType::Sudo(a)
    }
}

impl From<AteSessionGroup> for AteSessionType {
    fn from(a: AteSessionGroup) -> Self {
        AteSessionType::Group(a)
    }
}
