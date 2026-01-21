//! Host function registry implementation.
//!
//! Provides enum-based registry for host functions with support for both
//! JIT (function pointer) and emulator (ELF symbol) linking.

/// Enum identifying host functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostId {
    Debug,
    Println,
}

impl HostId {
    /// Get the symbol name for this host function.
    pub fn name(&self) -> &'static str {
        match self {
            HostId::Debug => "__host_debug",
            HostId::Println => "__host_println",
        }
    }

    /// Get all host IDs.
    pub fn all() -> &'static [HostId] {
        &[HostId::Debug, HostId::Println]
    }
}
