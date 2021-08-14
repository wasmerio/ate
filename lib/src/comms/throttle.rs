#[derive(Debug, Clone, Default)]
pub struct Throttle
{
    pub download_per_second: Option<u64>,
    pub upload_per_second: Option<u64>,
    pub read_only: bool,
}