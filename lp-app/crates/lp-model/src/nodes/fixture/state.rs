use alloc::vec::Vec;

/// Fixture node state - runtime values
#[derive(Debug, Clone)]
pub struct FixtureState {
    /// Lamp color values (RGB per lamp)
    pub lamp_colors: Vec<u8>,
}
