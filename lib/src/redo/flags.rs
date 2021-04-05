#[derive(Debug, Clone, Copy)]
pub struct OpenFlags
{
    pub truncate: bool,
    pub temporal: bool,
}

impl OpenFlags
{
    pub fn create() -> OpenFlags {
        OpenFlags {
            truncate: true,
            temporal: false,
        }
    }

    pub fn open() -> OpenFlags {
        OpenFlags {
            truncate: false,
            temporal: false,
        }
    }

    pub fn ethereal() -> OpenFlags {
        OpenFlags {
            truncate: false,
            temporal: true,
        }
    }
}