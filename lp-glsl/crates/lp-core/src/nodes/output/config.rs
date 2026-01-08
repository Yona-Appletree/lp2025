//! Output node configuration

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Output node types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum OutputNode {
    #[serde(rename = "gpio_strip")]
    GpioStrip {
        chip: String,
        gpio_pin: u32,
        count: u32,
    },
}
