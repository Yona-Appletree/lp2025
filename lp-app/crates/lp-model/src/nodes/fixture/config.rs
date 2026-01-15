use crate::nodes::{NodeConfig, NodeKind, NodeSpecifier};
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Color order for RGB channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorOrder {
    /// Red, Green, Blue
    Rgb,
    /// Green, Red, Blue
    Grb,
    /// Red, Blue, Green
    Rbg,
    /// Green, Blue, Red
    Gbr,
    /// Blue, Red, Green
    Brg,
    /// Blue, Green, Red
    Bgr,
}

impl ColorOrder {
    /// Get color order as string
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorOrder::Rgb => "rgb",
            ColorOrder::Grb => "grb",
            ColorOrder::Rbg => "rbg",
            ColorOrder::Gbr => "gbr",
            ColorOrder::Brg => "brg",
            ColorOrder::Bgr => "bgr",
        }
    }

    /// Get bytes per pixel (always 3 for RGB variants)
    pub fn bytes_per_pixel(&self) -> usize {
        3
    }

    /// Write RGB values to buffer in the correct order
    pub fn write_rgb(&self, buffer: &mut [u8], offset: usize, r: u8, g: u8, b: u8) {
        if offset + 3 > buffer.len() {
            return;
        }
        match self {
            ColorOrder::Rgb => {
                buffer[offset] = r;
                buffer[offset + 1] = g;
                buffer[offset + 2] = b;
            }
            ColorOrder::Grb => {
                buffer[offset] = g;
                buffer[offset + 1] = r;
                buffer[offset + 2] = b;
            }
            ColorOrder::Rbg => {
                buffer[offset] = r;
                buffer[offset + 1] = b;
                buffer[offset + 2] = g;
            }
            ColorOrder::Gbr => {
                buffer[offset] = g;
                buffer[offset + 1] = b;
                buffer[offset + 2] = r;
            }
            ColorOrder::Brg => {
                buffer[offset] = b;
                buffer[offset + 1] = r;
                buffer[offset + 2] = g;
            }
            ColorOrder::Bgr => {
                buffer[offset] = b;
                buffer[offset + 1] = g;
                buffer[offset + 2] = r;
            }
        }
    }
}

/// Fixture node configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureConfig {
    /// Output node specifier
    pub output_spec: NodeSpecifier,
    /// Texture node specifier
    pub texture_spec: NodeSpecifier,
    /// Mapping configuration (simplified for now)
    pub mapping: String, // todo!() - will be structured type later
    /// Lamp type (color order, etc.)
    pub lamp_type: String, // todo!() - will be enum later
    /// Color order for RGB channels
    pub color_order: ColorOrder,
    /// Transform matrix (4x4)
    pub transform: [[f32; 4]; 4], // todo!() - will be proper matrix type later
}

impl NodeConfig for FixtureConfig {
    fn kind(&self) -> NodeKind {
        NodeKind::Fixture
    }
    
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_fixture_config_kind() {
        let config = FixtureConfig {
            output_spec: NodeSpecifier::from("/src/out.output"),
            texture_spec: NodeSpecifier::from("/src/tex.texture"),
            mapping: "linear".to_string(),
            lamp_type: "rgb".to_string(),
            color_order: ColorOrder::Rgb,
            transform: [[1.0; 4]; 4],
        };
        assert_eq!(config.kind(), NodeKind::Fixture);
    }

    #[test]
    fn test_color_order_as_str() {
        assert_eq!(ColorOrder::Rgb.as_str(), "rgb");
        assert_eq!(ColorOrder::Grb.as_str(), "grb");
        assert_eq!(ColorOrder::Bgr.as_str(), "bgr");
    }

    #[test]
    fn test_color_order_bytes_per_pixel() {
        assert_eq!(ColorOrder::Rgb.bytes_per_pixel(), 3);
        assert_eq!(ColorOrder::Grb.bytes_per_pixel(), 3);
    }

    #[test]
    fn test_color_order_write_rgb() {
        let mut buffer = [0u8; 10];

        ColorOrder::Rgb.write_rgb(&mut buffer, 0, 100, 200, 255);
        assert_eq!(buffer[0], 100);
        assert_eq!(buffer[1], 200);
        assert_eq!(buffer[2], 255);

        ColorOrder::Grb.write_rgb(&mut buffer, 3, 100, 200, 255);
        assert_eq!(buffer[3], 200); // G first
        assert_eq!(buffer[4], 100); // R second
        assert_eq!(buffer[5], 255); // B third

        ColorOrder::Bgr.write_rgb(&mut buffer, 6, 100, 200, 255);
        assert_eq!(buffer[6], 255); // B first
        assert_eq!(buffer[7], 200); // G second
        assert_eq!(buffer[8], 100); // R third
    }

    #[test]
    fn test_color_order_write_rgb_bounds_check() {
        let mut buffer = [0u8; 2]; // Too small

        ColorOrder::Rgb.write_rgb(&mut buffer, 0, 100, 200, 255);
        // Should not panic, just return early
        // Buffer should be unchanged or partially written
    }
}
