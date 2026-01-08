//! Host function implementations and registry.

#[cfg(feature = "std")]
mod impls;
mod registry;

pub use registry::{HostId, declare_host_functions, get_host_function_pointer};

#[cfg(feature = "std")]
pub use impls::{__host_debug, __host_println};
