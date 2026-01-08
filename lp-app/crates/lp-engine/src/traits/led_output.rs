//! LED output abstraction trait

use crate::error::Error;

/// RGB pixel color (8-bit per component)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// RGBA pixel color (8-bit per component)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Platform-agnostic LED output trait
pub trait LedOutput {
    /// Write pixel data to the LED output
    ///
    /// The pixels are provided as a slice of bytes in the format specified by the
    /// output configuration (typically RGB or RGBA, 3 or 4 bytes per pixel).
    /// The length of the slice must match `get_pixel_count() * bytes_per_pixel`.
    fn write_pixels(&mut self, pixels: &[u8]) -> Result<(), Error>;

    /// Get the number of pixels (LEDs) in this output
    fn get_pixel_count(&self) -> usize;
}
