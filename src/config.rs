// SPDX-License-Identifier: MIT

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    /// CPU threshold for filtering processes (default: 50%)
    pub cpu_threshold: Option<u32>,
    /// Auto-refresh interval in seconds (default: 2)
    pub refresh_interval: Option<u32>,
}
