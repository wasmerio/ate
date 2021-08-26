use crate::trust::IntegrityMode;
use crate::crypto::AteHash;

#[derive(Debug, Clone, Copy)]
pub struct OpenFlags
{
    pub read_only: bool,
    pub truncate: bool,
    pub temporal: bool,
    pub integrity: IntegrityMode,
}

impl OpenFlags
{
    pub fn create_distributed() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: true,
            temporal: false,
            integrity: IntegrityMode::Distributed,
        }
    }
    
    pub fn create_centralized() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: true,
            temporal: false,
            integrity: IntegrityMode::Centralized(AteHash::generate()),
        }
    }

    pub fn open_distributed() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: false,
            integrity: IntegrityMode::Distributed,
        }
    }
    pub fn open_centralized() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: false,
            integrity: IntegrityMode::Centralized(AteHash::generate()),
        }
    }

    pub fn ethereal() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: true,
            integrity: IntegrityMode::Centralized(AteHash::generate()),
        }
    }
}