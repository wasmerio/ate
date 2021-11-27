use serde::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sleep {
    pub duration_ms: u128,
}