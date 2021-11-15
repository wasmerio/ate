#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub received: u64,
    pub sent: u64,
    pub requests: u64,
    pub chain_size: u64,
}
