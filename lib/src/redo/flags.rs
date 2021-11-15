use crate::spec::*;

#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read_only: bool,
    pub truncate: bool,
    pub temporal: bool,
    pub integrity: TrustMode,
}

impl OpenFlags {
    pub fn create_distributed() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: true,
            temporal: false,
            integrity: TrustMode::Distributed,
        }
    }

    pub fn create_centralized_server() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: true,
            temporal: false,
            integrity: TrustMode::Centralized(CentralizedRole::Server),
        }
    }

    pub fn create_centralized_client() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: true,
            temporal: false,
            integrity: TrustMode::Centralized(CentralizedRole::Client),
        }
    }

    pub fn open_distributed() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: false,
            integrity: TrustMode::Distributed,
        }
    }

    pub fn open_centralized_server() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: false,
            integrity: TrustMode::Centralized(CentralizedRole::Server),
        }
    }

    pub fn open_centralized_client() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: false,
            integrity: TrustMode::Centralized(CentralizedRole::Client),
        }
    }

    pub fn ethereal_distributed() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: true,
            integrity: TrustMode::Distributed,
        }
    }

    pub fn ethereal_centralized_server() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: true,
            integrity: TrustMode::Centralized(CentralizedRole::Server),
        }
    }

    pub fn ethereal_centralized_client() -> OpenFlags {
        OpenFlags {
            read_only: false,
            truncate: false,
            temporal: true,
            integrity: TrustMode::Centralized(CentralizedRole::Client),
        }
    }
}
