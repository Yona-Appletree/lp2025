use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Fixture node state - runtime values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureState {
    /// Lamp color values (RGB per lamp)
    pub lamp_colors: Vec<u8>,
}
