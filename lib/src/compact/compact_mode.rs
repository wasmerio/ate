use std::time::Duration;   

/// # Compaction State Machine
/// 
/// State machine that will trigger a compaction only when a particular set
/// of states has been reached.

// Specifies when a compaction event on a chain will occur.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompactMode
{
    // Compaction will never occur which effectivily means this chain is immutable
    Never,
    // Comapction will be triggered when the chain is modified in any way
    Modified,
    // Compaction will occur whenever a timer duration has been reached
    Timer(Duration),
    // Compaction will occur whenever growth exceeds a particular percentage
    GrowthFactor(f32),
    // Compaction will occur whenever growth exceeds a particular percentage or the timer is triggered
    GrowthFactorOrTimer {
        growth: f32,
        timer: Duration
    },
    // Compaction will occur whever the chain size increases by a certain absolute amount in bytes
    GrowthSize(u64),
    // Compaction will occur whever the chain size increases by a certain absolute amount in bytes or the timer is triggered
    GrowthSizeOrTimer {
        growth: u64,
        timer: Duration
    },
}

impl CompactMode
{
    pub fn with_timer_value(self: Self, val: Duration) -> Self {
        match self {
            CompactMode::Timer(_) => {
                CompactMode::Timer(val)
            },
            CompactMode::GrowthFactorOrTimer { growth, timer: _timer } => {
                CompactMode::GrowthFactorOrTimer { growth, timer: val }
            },
            CompactMode::GrowthSizeOrTimer { growth, timer: _timer } => {
                CompactMode::GrowthSizeOrTimer { growth, timer: val }
            },
            a => a,
        }
    }

    pub fn with_growth_factor(self: Self, val: f32) -> Self {
        match self {
            CompactMode::GrowthFactor(_) => {
                CompactMode::GrowthFactor(val)
            },
            CompactMode::GrowthFactorOrTimer { growth: _growth, timer } => {
                CompactMode::GrowthFactorOrTimer { growth: val, timer }
            },
            a => a,
        }
    }

    pub fn with_growth_size(self: Self, val: u64) -> Self {
        match self {
            CompactMode::GrowthSize(_) => {
                CompactMode::GrowthSize(val)
            },
            CompactMode::GrowthSizeOrTimer { growth: _growth, timer } => {
                CompactMode::GrowthSizeOrTimer { growth: val, timer }
            },
            a => a,
        }
    }
}

impl std::str::FromStr
for CompactMode
{
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "never" => Ok(CompactMode::Never),
            "immutable" => Ok(CompactMode::Never),
            "modified" => Ok(CompactMode::Modified),
            "timer" => Ok(CompactMode::Timer(Duration::from_secs(3600))),
            "factor" => Ok(CompactMode::GrowthFactor(0.2f32)),
            "size" => Ok(CompactMode::GrowthSize(104857600)),
            "factor-or-timer" => Ok(CompactMode::GrowthFactorOrTimer { growth: 0.2f32, timer: Duration::from_secs(3600) }),
            "size-or-timer" => Ok(CompactMode::GrowthSizeOrTimer { growth: 104857600, timer: Duration::from_secs(3600) }),
            _ => Err("valid values are 'never', 'modified', 'timer', 'factor', 'size', 'factor-or-timer', 'size-or-timer'"),
        }
    }
}