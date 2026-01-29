//! Pre-computed texture-to-fixture mapping utilities

use alloc::vec::Vec;
use lp_builtins::glsl::q32::types::q32::Q32;
use lp_model::FrameId;
use lp_model::nodes::fixture::mapping::{MappingConfig, PathSpec, RingOrder};

/// Sentinel value for channel index indicating no mapping (SKIP)
pub const CHANNEL_SKIP: u32 = 0x7FFF; // Max value for 15-bit channel index

/// Packed pixel-to-channel mapping entry
///
/// Bit layout:
/// - Bit 0: `has_more` flag (1 = more entries for this pixel follow)
/// - Bits 1-15: Channel index (15 bits, max 32767; CHANNEL_SKIP = no mapping)
/// - Bits 16-31: Contribution fraction (16 bits, stored as 65536 - contribution)
///   - 0 = 100% contribution
///   - 65535 = ~0% contribution
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PixelMappingEntry(u32);

impl PixelMappingEntry {
    /// Create a new entry
    ///
    /// # Arguments
    /// * `channel` - Channel index (0-32766, CHANNEL_SKIP reserved for sentinel)
    /// * `contribution` - Contribution fraction as Q32 (0.0 = 0%, 1.0 = 100%)
    /// * `has_more` - True if more entries follow for this pixel
    pub fn new(channel: u32, contribution: Q32, has_more: bool) -> Self {
        assert!(
            channel < CHANNEL_SKIP,
            "Channel index must be < CHANNEL_SKIP"
        );

        let continue_flag = if has_more { 1 } else { 0 };
        let channel_bits = (channel & 0x7FFF) << 1;

        // Contribution: Store (65535 - value) where value represents 0.0-1.0 in range 0-65535
        // Q32(65536) = 1.0, so we scale: value = (contribution.to_fixed() * 65535) / 65536
        // stored = 0 means 100% contribution, stored = 65535 means 0% contribution
        let contribution_raw = contribution.to_fixed();
        // Clamp to [0, 65536] and scale to [0, 65535]
        let value = if contribution_raw >= 65536 {
            65535
        } else {
            ((contribution_raw as i64 * 65535) / 65536) as u32
        };
        let stored_contribution = 65535u32.saturating_sub(value);
        let contribution_bits = (stored_contribution & 0xFFFF) << 16;

        Self(continue_flag | channel_bits | contribution_bits)
    }

    /// Create SKIP sentinel entry (no mapping for this pixel)
    pub fn skip() -> Self {
        Self((CHANNEL_SKIP << 1) | 1) // has_more = true, channel = SKIP
    }

    /// Extract channel index
    pub fn channel(&self) -> u32 {
        (self.0 >> 1) & 0x7FFF
    }

    /// Extract contribution as Q32 (0.0 = 0%, 1.0 = 100%)
    /// Decodes: stored = 0 means 100% (Q32::ONE), stored = 65535 means 0% (Q32::ZERO)
    pub fn contribution(&self) -> Q32 {
        let stored = (self.0 >> 16) & 0xFFFF;
        if stored == 0 {
            // Stored 0 = 100% contribution = Q32::ONE (65536)
            Q32::ONE
        } else {
            // Stored 1-65535 maps to contribution 0-0.99998
            // We need to map (65535 - stored) from range [0, 65534] to Q32 range [0, 65535]
            // Scale: (65535 - stored) * 65536 / 65535
            let fractional_part = 65535u32 - stored;
            // Convert to Q32: scale from [0, 65534] to [0, 65535] in Q32 space
            // Use i64 to avoid overflow: (fractional_part * 65536) / 65535
            let q32_value = ((fractional_part as i64 * 65536) / 65535) as i32;
            Q32::from_fixed(q32_value)
        }
    }

    /// Check if more entries follow for this pixel
    pub fn has_more(&self) -> bool {
        (self.0 & 1) != 0
    }

    /// Check if this is the SKIP sentinel
    pub fn is_skip(&self) -> bool {
        self.channel() == CHANNEL_SKIP
    }

    /// Get raw u32 value
    pub fn to_raw(&self) -> u32 {
        self.0
    }

    /// Create from raw u32
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

/// Compute the area overlap between a circle and a pixel square
///
/// Uses 8x8 subdivision (64 sub-pixels) to estimate overlap area.
/// Returns normalized weight (0.0 to 1.0) representing how much of the pixel
/// is covered by the circle.
///
/// # Arguments
/// * `circle_center_x` - Circle center X in pixel coordinates
/// * `circle_center_y` - Circle center Y in pixel coordinates  
/// * `circle_radius` - Circle radius in pixels
/// * `pixel_x` - Pixel X coordinate (integer)
/// * `pixel_y` - Pixel Y coordinate (integer)
///
/// # Returns
/// Normalized weight (0.0 to 1.0) representing pixel coverage
pub fn circle_pixel_overlap(
    circle_center_x: f32,
    circle_center_y: f32,
    circle_radius: f32,
    pixel_x: u32,
    pixel_y: u32,
) -> f32 {
    const SUBDIVISIONS: u32 = 8;
    const TOTAL_SAMPLES: f32 = (SUBDIVISIONS * SUBDIVISIONS) as f32;

    // Pixel bounds
    let px_min = pixel_x as f32;
    let py_min = pixel_y as f32;

    // Sub-pixel size
    let sub_pixel_size = 1.0 / SUBDIVISIONS as f32;

    // Count sub-pixels within circle
    let mut count = 0u32;

    for i in 0..SUBDIVISIONS {
        for j in 0..SUBDIVISIONS {
            // Sub-pixel center coordinates
            let sub_x = px_min + (i as f32 + 0.5) * sub_pixel_size;
            let sub_y = py_min + (j as f32 + 0.5) * sub_pixel_size;

            // Distance from circle center to sub-pixel center
            let dx = sub_x - circle_center_x;
            let dy = sub_y - circle_center_y;
            let dist_sq = dx * dx + dy * dy;

            // Check if sub-pixel center is within circle
            if dist_sq <= circle_radius * circle_radius {
                count += 1;
            }
        }
    }

    // Normalize: count / total_samples gives coverage fraction
    count as f32 / TOTAL_SAMPLES
}

/// Pre-computed texture-to-fixture mapping
///
/// Contains a flat list of `PixelMappingEntry` values ordered by pixel (x, y).
/// Each pixel's entries are consecutive, with the last entry having `has_more = false`.
/// Pixels with no contributions have a SKIP sentinel entry.
#[derive(Debug, Clone)]
pub struct PrecomputedMapping {
    /// Flat list of mapping entries, ordered by pixel (x, y)
    pub entries: Vec<PixelMappingEntry>,
    /// Texture width (for validation)
    pub texture_width: u32,
    /// Texture height (for validation)
    pub texture_height: u32,
    /// FrameId when this mapping was computed
    pub mapping_data_ver: FrameId,
}

impl PrecomputedMapping {
    /// Create a new empty mapping
    pub fn new(texture_width: u32, texture_height: u32, mapping_data_ver: FrameId) -> Self {
        Self {
            entries: Vec::new(),
            texture_width,
            texture_height,
            mapping_data_ver,
        }
    }

    /// Check if mapping is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Get total number of pixels
    pub fn pixel_count(&self) -> u32 {
        self.texture_width * self.texture_height
    }
}

/// Mapping point representing a single LED sampling location
/// (Temporary struct for pre-computation, matches runtime::MappingPoint)
struct MappingPoint {
    channel: u32,
    center: [f32; 2], // Texture space coordinates [0, 1]
    radius: f32,      // Normalized radius [0, 1]
}

/// Compute pre-computed mapping from configuration
///
/// # Arguments
/// * `config` - Mapping configuration
/// * `texture_width` - Texture width in pixels
/// * `texture_height` - Texture height in pixels
/// * `mapping_data_ver` - FrameId for version tracking
///
/// # Returns
/// PrecomputedMapping with entries ordered by pixel (x, y)
pub fn compute_mapping(
    config: &MappingConfig,
    texture_width: u32,
    texture_height: u32,
    mapping_data_ver: FrameId,
) -> PrecomputedMapping {
    let mut mapping = PrecomputedMapping::new(texture_width, texture_height, mapping_data_ver);

    match config {
        MappingConfig::PathPoints {
            paths,
            sample_diameter,
        } => {
            // First pass: collect all mapping points (circles)
            let mut mapping_points = Vec::new();
            let mut channel_offset = 0u32;

            for path_spec in paths {
                let points = match path_spec {
                    PathSpec::RingArray {
                        center,
                        diameter,
                        start_ring_inclusive,
                        end_ring_exclusive,
                        ring_lamp_counts,
                        offset_angle,
                        order,
                    } => generate_ring_array_points_for_precompute(
                        *center,
                        *diameter,
                        *start_ring_inclusive,
                        *end_ring_exclusive,
                        ring_lamp_counts,
                        *offset_angle,
                        *order,
                        *sample_diameter,
                        texture_width,
                        texture_height,
                        channel_offset,
                    ),
                };

                channel_offset += points.len() as u32;
                mapping_points.extend(points);
            }

            // Second pass: for each pixel, compute contributions from all circles
            let mut pixel_contributions: Vec<Vec<(u32, f32)>> =
                Vec::with_capacity((texture_width * texture_height) as usize);
            pixel_contributions.resize((texture_width * texture_height) as usize, Vec::new());
            
            // Track total weight per channel for normalization
            let mut channel_totals: Vec<f32> = Vec::new();
            let max_channel = mapping_points.iter().map(|p| p.channel).max().unwrap_or(0);
            channel_totals.resize((max_channel + 1) as usize, 0.0);

            for mapping_point in &mapping_points {
                // Convert normalized coordinates to pixel coordinates
                let center_x = mapping_point.center[0] * texture_width as f32;
                let center_y = mapping_point.center[1] * texture_height as f32;
                // Convert normalized radius to pixel radius
                let radius = mapping_point.radius * texture_width.max(texture_height) as f32;

                // Find pixels that might overlap with this circle
                let min_x = ((center_x - radius).floor() as i32).max(0) as u32;
                let max_x =
                    ((center_x + radius).ceil() as i32).min(texture_width as i32 - 1) as u32;
                let min_y = ((center_y - radius).floor() as i32).max(0) as u32;
                let max_y =
                    ((center_y + radius).ceil() as i32).min(texture_height as i32 - 1) as u32;

                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let weight = circle_pixel_overlap(center_x, center_y, radius, x, y);
                        if weight > 0.0 {
                            let pixel_idx = (y * texture_width + x) as usize;
                            pixel_contributions[pixel_idx].push((mapping_point.channel, weight));
                            // Accumulate total weight per channel
                            channel_totals[mapping_point.channel as usize] += weight;
                        }
                    }
                }
            }

            // Third pass: normalize weights per-channel and build entries
            // Each channel's total contribution from all pixels should sum to 1.0
            for y in 0..texture_height {
                for x in 0..texture_width {
                    let pixel_idx = (y * texture_width + x) as usize;
                    let contributions = &pixel_contributions[pixel_idx];

                    if contributions.is_empty() {
                        // No contributions - add SKIP entry
                        mapping.entries.push(PixelMappingEntry::skip());
                    } else {
                        // Normalize weights per-channel: divide by channel total
                        // This ensures each channel's total contribution from all pixels = 1.0
                        let normalized: Vec<(u32, f32)> = contributions
                            .iter()
                            .map(|(ch, w)| {
                                let channel_total = channel_totals[*ch as usize];
                                if channel_total > 0.0 {
                                    (*ch, *w / channel_total)
                                } else {
                                    (*ch, 0.0)
                                }
                            })
                            .collect();

                        // Add entries (last one has has_more = false)
                        for (idx, (channel, weight)) in normalized.iter().enumerate() {
                            let has_more = idx < normalized.len() - 1;
                            let contribution_q32 = Q32::from_f32(*weight);
                            mapping.entries.push(PixelMappingEntry::new(
                                *channel,
                                contribution_q32,
                                has_more,
                            ));
                        }
                    }
                }
            }
        }
    }

    mapping
}

/// Generate mapping points from RingArray path specification
/// (Adapted from runtime.rs for pre-computation)
fn generate_ring_array_points_for_precompute(
    center: (f32, f32),
    diameter: f32,
    start_ring_inclusive: u32,
    end_ring_exclusive: u32,
    ring_lamp_counts: &Vec<u32>,
    offset_angle: f32,
    order: RingOrder,
    sample_diameter: f32,
    texture_width: u32,
    texture_height: u32,
    channel_offset: u32,
) -> Vec<MappingPoint> {
    let (center_x, center_y) = center;
    let start_ring = start_ring_inclusive;
    let end_ring = end_ring_exclusive;

    // Calculate max ring index for spacing
    let max_ring_index = if end_ring > start_ring {
        (end_ring - start_ring - 1) as f32
    } else {
        0.0
    };

    // Convert sample_diameter (pixels) to normalized radius
    let max_dimension = texture_width.max(texture_height) as f32;
    let normalized_radius = (sample_diameter / 2.0) / max_dimension;

    // Determine ring processing order
    let ring_indices: Vec<u32> = match order {
        RingOrder::InnerFirst => (start_ring..end_ring).collect(),
        RingOrder::OuterFirst => (start_ring..end_ring).rev().collect(),
    };

    let mut points = Vec::new();
    let mut current_channel = channel_offset;

    for ring_index in ring_indices {
        // Calculate ring radius (even spacing)
        let ring_radius = if max_ring_index > 0.0 {
            (diameter / 2.0) * ((ring_index - start_ring) as f32 / max_ring_index)
        } else {
            0.0
        };

        // Get lamp count for this ring
        let lamp_count = ring_lamp_counts
            .get(ring_index as usize)
            .copied()
            .unwrap_or(0);

        // Generate points for each lamp in the ring
        for lamp_index in 0..lamp_count {
            let angle = (2.0 * core::f32::consts::PI * lamp_index as f32 / lamp_count as f32)
                + offset_angle;

            let x = center_x + ring_radius * angle.cos();
            let y = center_y + ring_radius * angle.sin();

            // Clamp to [0, 1] range
            let x = x.max(0.0).min(1.0);
            let y = y.max(0.0).min(1.0);

            points.push(MappingPoint {
                channel: current_channel,
                center: [x, y],
                radius: normalized_radius,
            });

            current_channel += 1;
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_entry() {
        let entry = PixelMappingEntry::new(5, Q32::from_f32(0.5), false);
        assert_eq!(entry.channel(), 5);
        assert!((entry.contribution().to_f32() - 0.5).abs() < 0.001);
        assert!(!entry.has_more());
        assert!(!entry.is_skip());
    }

    #[test]
    fn test_full_contribution() {
        // 0 stored = 100% contribution
        let entry = PixelMappingEntry::new(0, Q32::from_f32(1.0), false);
        assert_eq!(entry.channel(), 0);
        assert!((entry.contribution().to_f32() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_zero_contribution() {
        let entry = PixelMappingEntry::new(0, Q32::from_f32(0.0), false);
        assert_eq!(entry.channel(), 0);
        assert!(entry.contribution().to_f32() < 0.01);
    }

    #[test]
    fn test_has_more_flag() {
        let entry_more = PixelMappingEntry::new(1, Q32::from_f32(0.5), true);
        assert!(entry_more.has_more());

        let entry_last = PixelMappingEntry::new(1, Q32::from_f32(0.5), false);
        assert!(!entry_last.has_more());
    }

    #[test]
    fn test_skip_sentinel() {
        let skip = PixelMappingEntry::skip();
        assert!(skip.is_skip());
        assert_eq!(skip.channel(), CHANNEL_SKIP);
        assert!(skip.has_more()); // SKIP entries have has_more = true
    }

    #[test]
    fn test_round_trip() {
        let original = PixelMappingEntry::new(42, Q32::from_f32(0.75), true);
        let raw = original.to_raw();
        let reconstructed = PixelMappingEntry::from_raw(raw);

        assert_eq!(original.channel(), reconstructed.channel());
        assert!(
            (original.contribution().to_f32() - reconstructed.contribution().to_f32()).abs() < 0.01
        );
        assert_eq!(original.has_more(), reconstructed.has_more());
    }

    #[test]
    fn test_max_channel() {
        let entry = PixelMappingEntry::new(CHANNEL_SKIP - 1, Q32::from_f32(0.5), false);
        assert_eq!(entry.channel(), CHANNEL_SKIP - 1);
    }
}

#[cfg(test)]
mod overlap_tests {
    use super::*;

    #[test]
    fn test_full_overlap() {
        // Circle completely covers pixel
        let weight = circle_pixel_overlap(0.5, 0.5, 1.0, 0, 0);
        assert!(
            weight >= 0.95,
            "Full overlap should be close to 1.0, got {}",
            weight
        );
    }

    #[test]
    fn test_no_overlap() {
        // Circle far from pixel
        let weight = circle_pixel_overlap(10.0, 10.0, 0.5, 0, 0);
        assert!(
            weight < 0.01,
            "No overlap should be close to 0.0, got {}",
            weight
        );
    }

    #[test]
    fn test_partial_overlap() {
        // Circle partially overlaps pixel (center at edge)
        let weight = circle_pixel_overlap(0.0, 0.5, 0.5, 0, 0);
        assert!(
            weight > 0.0 && weight < 1.0,
            "Partial overlap should be between 0 and 1, got {}",
            weight
        );
    }

    #[test]
    fn test_circle_at_pixel_center() {
        // Circle centered on pixel
        let weight = circle_pixel_overlap(0.5, 0.5, 0.3, 0, 0);
        assert!(weight > 0.0 && weight <= 1.0);
    }

    #[test]
    fn test_small_circle() {
        // Very small circle
        let weight = circle_pixel_overlap(0.5, 0.5, 0.1, 0, 0);
        assert!(weight > 0.0 && weight < 1.0);
    }

    #[test]
    fn test_large_circle() {
        // Very large circle covering multiple pixels
        let weight = circle_pixel_overlap(0.5, 0.5, 10.0, 0, 0);
        assert!(weight >= 0.95, "Large circle should cover pixel completely");
    }

    #[test]
    fn test_edge_pixel() {
        // Circle at edge of texture
        let weight = circle_pixel_overlap(0.0, 0.0, 0.5, 0, 0);
        assert!(weight > 0.0 && weight <= 1.0);
    }

    #[test]
    fn test_symmetry() {
        // Overlap should be symmetric
        let w1 = circle_pixel_overlap(1.5, 0.5, 0.5, 1, 0);
        let w2 = circle_pixel_overlap(0.5, 1.5, 0.5, 0, 1);
        // Should be similar (not necessarily equal due to discretization)
        assert!(
            (w1 - w2).abs() < 0.1,
            "Symmetry check failed: {} vs {}",
            w1,
            w2
        );
    }
}

#[cfg(test)]
mod precomputed_mapping_tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let mapping = PrecomputedMapping::new(100, 200, FrameId::new(42));
        assert!(mapping.is_empty());
        assert_eq!(mapping.len(), 0);
        assert_eq!(mapping.texture_width, 100);
        assert_eq!(mapping.texture_height, 200);
        assert_eq!(mapping.mapping_data_ver, FrameId::new(42));
        assert_eq!(mapping.pixel_count(), 20000);
    }

    #[test]
    fn test_with_entries() {
        let mut mapping = PrecomputedMapping::new(10, 10, FrameId::new(1));
        mapping
            .entries
            .push(PixelMappingEntry::new(0, Q32::from_f32(1.0), false));
        mapping.entries.push(PixelMappingEntry::skip());

        assert!(!mapping.is_empty());
        assert_eq!(mapping.len(), 2);
    }
}
