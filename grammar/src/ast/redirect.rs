use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct Redirect {
    pub fd: i32,
    pub filename: String,
    pub op: RedirectionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RedirectionType {
    TO,      // fd > fname
    CLOBBER, // fd >| fname
    FROM,    // fd < fname
    FROMTO,  // fd <> fname
    APPEND,  // fd >> fname
    TOFD,    // fd >& dupfd
    FROMFD,  // fd <& dupfd
}

impl FromStr for RedirectionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">" => Ok(RedirectionType::TO),
            ">|" => Ok(RedirectionType::CLOBBER),
            "<" => Ok(RedirectionType::FROM),
            "<>" => Ok(RedirectionType::FROMTO),
            ">>" => Ok(RedirectionType::APPEND),
            ">&" => Ok(RedirectionType::TOFD),
            "<&" => Ok(RedirectionType::FROMFD),
            _ => Err(()),
        }
    }
}

impl RedirectionType {
    pub fn read(&self) -> bool {
        match self {
            RedirectionType::FROM => true,
            RedirectionType::FROMTO => true,
            RedirectionType::FROMFD => true,
            _ => false,
        }
    }

    pub fn write(&self) -> bool {
        match self {
            RedirectionType::TO => true,
            RedirectionType::CLOBBER => true,
            RedirectionType::FROMTO => true,
            RedirectionType::APPEND => true,
            RedirectionType::TOFD => true,
            _ => false,
        }
    }

    pub fn duplicate(&self) -> bool {
        match self {
            RedirectionType::TOFD => true,
            RedirectionType::FROMFD => true,
            _ => false,
        }
    }

    pub fn append(&self) -> bool {
        match self {
            RedirectionType::APPEND => true,
            _ => false,
        }
    }

    pub fn clobber(&self) -> bool {
        match self {
            RedirectionType::CLOBBER => true,
            _ => false,
        }
    }
}
