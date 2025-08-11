use std::time::Duration;

use serde::Deserialize;
use derive_builder::Builder;


pub const NO_TIMEOUT: Duration = Duration::from_secs(300);
pub const NO_BANDWITH_LIMIT: u64 = 1_000_000_000_000_u64;

#[derive(Builder,Deserialize,Clone)]
pub struct RelayPolicy {
    #[serde(with = "humantime_serde")]
    pub idle_timeout: Duration,
    pub min_rate_bpm:u64,
    pub max_rate_bps:u64,
}
