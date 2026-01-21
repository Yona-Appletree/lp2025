//! Section loading for object files.

extern crate alloc;

use crate::debug;
use ::object::{Object, ObjectSection};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::layout::ObjectLayout;

/// Information about where object file sections were placed.
pub struct ObjectSectionPlacement {
    /// Start address where .text section was placed
    pub text_start: u32,
    /// Size of .text section
    pub text_size: usize,
    /// Start address where .data section was placed (relative to RAM_START)
    pub data_start: u32,
    /// Size of .data section
    pub data_size: usize,
}

/// Load object file sections into memory buffers.
///
/// Copies object file sections into the base executable's code/ram buffers
/// at the specified placement addresses.
///
/// # Arguments
///
/// * `obj` - The object file to load sections from
/// * `code` - Mutable reference to code buffer (will be extended if needed)
/// * `ram` - Mutable reference to RAM buffer (will be extended if needed)
/// * `layout` - Layout information specifying where to place sections
///
/// # Returns
///
/// Information about where sections were placed, or an error if loading fails.
pub fn load_object_sections(
    obj: &::object::File,
    code: &mut Vec<u8>,
    ram: &mut Vec<u8>,
    layout: &ObjectLayout,
) -> Result<ObjectSectionPlacement, String> {
    debug!("=== Loading object file sections ===");

    let text_start = layout.text_placement;
    let mut text_size = 0usize;
    let data_start = layout.data_placement;
    let mut data_size = 0usize;

    // Iterate through sections and load them
    for section in obj.sections() {
        let section_name = match section.name() {
            Ok(name) => name,
            Err(_) => continue,
        };

        // Skip debug sections
        if section_name.starts_with(".debug_") || section_name.starts_with(".zdebug_") {
            continue;
        }

        let section_kind = section.kind();
        let section_size = section.size() as usize;

        // Skip sections with no data (except .bss which we'll handle separately)
        if section_size == 0 && section_kind != ::object::SectionKind::UninitializedData {
            continue;
        }

        match section_name {
            ".text" => {
                // Load .text section into code buffer
                let data = section
                    .data()
                    .map_err(|e| format!("Failed to read .text section data: {}", e))?;

                if !data.is_empty() {
                    // Ensure code buffer is large enough
                    let required_size = (text_start as usize) + data.len();
                    if required_size > code.len() {
                        code.resize(required_size, 0);
                    }

                    // Copy section data
                    code[text_start as usize..text_start as usize + data.len()]
                        .copy_from_slice(data);

                    text_size = data.len();
                }
            }
            ".data" => {
                // Load .data section into RAM buffer
                let data = section
                    .data()
                    .map_err(|e| format!("Failed to read .data section data: {}", e))?;

                if !data.is_empty() {
                    // Ensure RAM buffer is large enough
                    let required_size = (data_start as usize) + data.len();
                    if required_size > ram.len() {
                        ram.resize(required_size, 0);
                    }

                    // Copy section data
                    ram[data_start as usize..data_start as usize + data.len()]
                        .copy_from_slice(data);

                    data_size = data.len();
                }
            }
            ".rodata" => {
                // Load .rodata section into code buffer (after .text)
                let data = section
                    .data()
                    .map_err(|e| format!("Failed to read .rodata section data: {}", e))?;

                if !data.is_empty() {
                    // Place .rodata after .text
                    let rodata_start = text_start + text_size as u32;
                    let rodata_start_aligned = (rodata_start + 3) & !3; // Align to 4 bytes

                    // Ensure code buffer is large enough
                    let required_size = (rodata_start_aligned as usize) + data.len();
                    if required_size > code.len() {
                        code.resize(required_size, 0);
                    }

                    // Copy section data
                    code[rodata_start_aligned as usize..rodata_start_aligned as usize + data.len()]
                        .copy_from_slice(data);
                }
            }
            ".bss" => {
                // Zero-initialize .bss section in RAM buffer (after .data)
                if section_size > 0 {
                    // Place .bss after .data
                    let bss_start = data_start + data_size as u32;
                    let bss_start_aligned = (bss_start + 3) & !3; // Align to 4 bytes

                    // Ensure RAM buffer is large enough
                    let required_size = (bss_start_aligned as usize) + section_size;
                    if required_size > ram.len() {
                        ram.resize(required_size, 0);
                    } else {
                        // Zero-initialize the .bss region
                        ram[bss_start_aligned as usize..bss_start_aligned as usize + section_size]
                            .fill(0);
                    }
                }
            }
            _ => {
                // Skip other sections for now
            }
        }
    }

    debug!(
        "Object section loading complete: .text at 0x{:x} ({} bytes), .data at offset 0x{:x} ({} bytes)",
        text_start, text_size, data_start, data_size
    );

    Ok(ObjectSectionPlacement {
        text_start,
        text_size,
        data_start,
        data_size,
    })
}
