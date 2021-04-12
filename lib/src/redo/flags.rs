use crate::trust::IntegrityMode;

#[derive(Debug, Clone, Copy)]
pub struct OpenFlags
{
    pub truncate: bool,
    pub temporal: bool,
    pub integrity: IntegrityMode,
}

impl OpenFlags
{
    pub fn create_distributed() -> OpenFlags {
        OpenFlags {
            truncate: true,
            temporal: false,
            integrity: IntegrityMode::Distributed,
        }
    }
    
    pub fn create_centralized() -> OpenFlags {
        OpenFlags {
            truncate: true,
            temporal: false,
            integrity: IntegrityMode::Centralized,
        }
    }

    pub fn open_distributed() -> OpenFlags {
        OpenFlags {
            truncate: false,
            temporal: false,
            integrity: IntegrityMode::Distributed,
        }
    }
    pub fn open_centralized() -> OpenFlags {
        OpenFlags {
            truncate: false,
            temporal: false,
            integrity: IntegrityMode::Centralized,
        }
    }

    pub fn ethereal() -> OpenFlags {
        OpenFlags {
            truncate: false,
            temporal: true,
            integrity: IntegrityMode::Centralized,
        }
    }
}