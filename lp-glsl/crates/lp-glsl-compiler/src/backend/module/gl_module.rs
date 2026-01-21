//! GLSL Module - owns the actual Cranelift Module

use crate::backend::module::gl_func::GlFunc;
use crate::backend::target::Target;
use crate::error::{ErrorCode, GlslError};
use crate::frontend::semantic::functions::{FunctionRegistry, FunctionSignature};
use crate::frontend::src_loc::GlSourceMap;
use crate::frontend::src_loc_manager::SourceLocManager;
#[cfg(feature = "std")]
use alloc::boxed::Box;
use alloc::string::String;
use cranelift_jit::JITModule;
use cranelift_module::Module;
#[cfg(feature = "emulator")]
use cranelift_object::ObjectModule;
use hashbrown::HashMap;

/// GLSL Module - owns the actual Cranelift Module
pub struct GlModule<M: Module> {
    pub target: Target, // Semantic target, not technical spec
    pub fns: HashMap<String, GlFunc>,
    module: M, // PRIVATE - only accessible via internal methods
    // Metadata fields
    pub function_registry: FunctionRegistry,
    pub glsl_signatures: HashMap<String, FunctionSignature>,
    pub source_text: String,
    pub source_loc_manager: SourceLocManager,
    pub source_map: GlSourceMap,
}

// Separate constructors for each Module type (Rust needs concrete types)
impl GlModule<JITModule> {
    /// Create new GlModule with JITModule from HostJit target
    pub fn new_jit(mut target: Target) -> Result<Self, GlslError> {
        match &target {
            Target::HostJit { .. } => {
                let mut builder = target.create_module_builder()?;
                // Add builtin and host symbol lookup function before creating module
                {
                    use crate::backend::builtins::registry::{BuiltinId, get_function_pointer};
                    #[cfg(feature = "std")]
                    use crate::backend::host::{HostId, get_host_function_pointer};
                    match &mut builder {
                        crate::backend::target::builder::ModuleBuilder::JIT(jit_builder) => {
                            // Create lookup function that returns builtin and host function pointers
                            // This works in both std and no_std - iterate through builtins directly
                            jit_builder.symbol_lookup_fn(Box::new(
                                move |name: &str| -> Option<*const u8> {
                                    // Check builtins first
                                    for builtin in BuiltinId::all() {
                                        if builtin.name() == name {
                                            return Some(get_function_pointer(*builtin));
                                        }
                                    }
                                    // Check host functions (only in std mode)
                                    #[cfg(feature = "std")]
                                    {
                                        for host in HostId::all() {
                                            if host.name() == name {
                                                return Some(get_host_function_pointer(*host));
                                            }
                                        }
                                    }
                                    None
                                },
                            ));
                        }
                        #[cfg(feature = "emulator")]
                        crate::backend::target::builder::ModuleBuilder::Object(_) => {
                            return Err(GlslError::new(
                                crate::error::ErrorCode::E0400,
                                "HostJit target must create JIT builder",
                            ));
                        }
                    }
                }
                let mut module = match builder {
                    crate::backend::target::builder::ModuleBuilder::JIT(jit_builder) => {
                        JITModule::new(jit_builder)
                    }
                    #[cfg(feature = "emulator")]
                    crate::backend::target::builder::ModuleBuilder::Object(_) => {
                        return Err(GlslError::new(
                            crate::error::ErrorCode::E0400,
                            "HostJit target cannot create Object builder",
                        ));
                    }
                };

                // Declare builtin functions when module is created
                {
                    use crate::backend::builtins::declare_builtins;
                    declare_builtins(&mut module)?;
                }

                Ok(Self {
                    target,
                    fns: HashMap::new(),
                    module,
                    function_registry: FunctionRegistry::new(),
                    glsl_signatures: HashMap::new(),
                    source_text: String::new(),
                    source_loc_manager: SourceLocManager::new(),
                    source_map: GlSourceMap::new(),
                })
            }
            _ => Err(GlslError::new(
                ErrorCode::E0400,
                "Target is not a JIT target",
            )),
        }
    }

    /// Create new GlModule with same target (for transformations)
    pub fn new_with_target(target: Target) -> Result<Self, GlslError> {
        Self::new_jit(target)
    }
}

#[cfg(feature = "emulator")]
impl GlModule<ObjectModule> {
    /// Create new GlModule with ObjectModule from Rv32Emu target
    pub fn new_object(mut target: Target) -> Result<Self, GlslError> {
        match &target {
            Target::Rv32Emu { .. } => {
                let builder = target.create_module_builder()?;
                let mut module = match builder {
                    crate::backend::target::builder::ModuleBuilder::Object(obj_builder) => {
                        ObjectModule::new(obj_builder)
                    }
                    _ => return Err(GlslError::new(ErrorCode::E0400, "Expected Object builder")),
                };

                // Declare builtin functions when module is created
                {
                    use crate::backend::builtins::declare_builtins;
                    declare_builtins(&mut module)?;
                }

                // Declare host functions when module is created (for emulator)
                // Note: Host functions use fmt::Arguments which can't be represented in Cranelift,
                // so these declarations are placeholders. The actual functions are linked from
                // lp-builtins-app and will be resolved by the linker.
                #[cfg(feature = "std")]
                {
                    use crate::backend::host::declare_host_functions;
                    // Only declare if std is available (host functions require std)
                    let _ = declare_host_functions(&mut module);
                    // Ignore errors - host functions may not be usable from compiled code
                }

                Ok(Self {
                    target,
                    fns: HashMap::new(),
                    module,
                    function_registry: FunctionRegistry::new(),
                    glsl_signatures: HashMap::new(),
                    source_text: String::new(),
                    source_loc_manager: SourceLocManager::new(),
                    source_map: GlSourceMap::new(),
                })
            }
            _ => Err(GlslError::new(
                ErrorCode::E0400,
                "Target is not an object target",
            )),
        }
    }

    /// Create new GlModule with same target (for transformations)
    pub fn new_with_target(target: Target) -> Result<Self, GlslError> {
        Self::new_object(target)
    }
}

impl<M: Module> GlModule<M> {
    /// Get function metadata by name
    pub fn get_func(&self, name: &str) -> Option<&GlFunc> {
        self.fns.get(name)
    }

    /// Get a FuncRef for a builtin function that can be used in function building.
    ///
    /// This handles the differences between JIT and ObjectModule:
    /// - For JIT: Uses UserExternalName with FuncId, resolved via symbol_lookup_fn
    /// - For ObjectModule: Uses FuncId from module declarations, generates direct call
    ///
    /// The builtin must have been declared via `declare_builtins` before calling this.
    pub fn get_builtin_func_ref(
        &mut self,
        builtin: crate::backend::builtins::registry::BuiltinId,
        func: &mut cranelift_codegen::ir::Function,
    ) -> Result<cranelift_codegen::ir::FuncRef, GlslError> {
        use cranelift_module::FuncOrDataId;

        let name = builtin.name();
        let func_id = self
            .module
            .declarations()
            .get_name(name)
            .and_then(|id| match id {
                FuncOrDataId::Func(fid) => Some(fid),
                FuncOrDataId::Data(_) => None,
            })
            .ok_or_else(|| {
                GlslError::new(
                    crate::error::ErrorCode::E0400,
                    format!(
                        "Builtin function '{}' not found in module declarations. Ensure declare_builtins() was called.",
                        name
                    ),
                )
            })?;

        // Use declare_func_in_func which handles both JIT and ObjectModule correctly:
        // - For JIT: Creates UserExternalName that will be resolved via symbol_lookup_fn
        // - For ObjectModule: Creates UserExternalName that maps to the symbol name for linker resolution
        // The colocated flag is determined by the linkage (Import -> false, but that's handled internally)
        Ok(self.module.declare_func_in_func(func_id, func))
    }

    /// Get a FuncRef for a builtin function by FuncId (for use during transformations).
    ///
    /// This is a lower-level version that takes a FuncId directly, useful when you
    /// already have the FuncId from func_id_map during transformations.
    ///
    /// This handles the differences between JIT and ObjectModule correctly.
    pub fn get_builtin_func_ref_by_id(
        &mut self,
        func_id: cranelift_module::FuncId,
        func: &mut cranelift_codegen::ir::Function,
    ) -> cranelift_codegen::ir::FuncRef {
        // Use declare_func_in_func which handles both JIT and ObjectModule correctly
        self.module.declare_func_in_func(func_id, func)
    }

    /// Add a function to this module
    ///
    /// Declares the function in the Module and stores the Function IR.
    /// The function is NOT compiled yet - that happens in build_executable().
    ///
    /// Validates that the Function signature matches the provided signature.
    pub fn add_function(
        &mut self,
        name: &str,
        linkage: cranelift_module::Linkage,
        sig: cranelift_codegen::ir::Signature,
        func: cranelift_codegen::ir::Function,
    ) -> Result<cranelift_module::FuncId, GlslError> {
        // Validate signature matches
        if func.signature != sig {
            return Err(GlslError::new(
                ErrorCode::E0400,
                format!("Function signature mismatch for '{}'", name),
            ));
        }

        // Declare in Module
        let func_id = self
            .module
            .declare_function(name, linkage, &sig)
            .map_err(|e| {
                GlslError::new(
                    ErrorCode::E0400,
                    format!("Failed to declare function '{}': {}", name, e),
                )
            })?;

        // Store Function IR
        self.fns.insert(
            String::from(name),
            GlFunc {
                name: String::from(name),
                clif_sig: sig,
                func_id,
                function: func,
            },
        );

        Ok(func_id)
    }

    /// Declare a function without providing the body yet (forward declaration)
    ///
    /// Useful for cross-function calls where the callee is defined later.
    /// Note: The function must be defined later using `add_function` with the same name.
    pub fn declare_function(
        &mut self,
        name: &str,
        linkage: cranelift_module::Linkage,
        sig: cranelift_codegen::ir::Signature,
    ) -> Result<cranelift_module::FuncId, GlslError> {
        // Declare in Module
        let func_id = self
            .module
            .declare_function(name, linkage, &sig)
            .map_err(|e| {
                GlslError::new(
                    ErrorCode::E0400,
                    format!("Failed to declare function '{}': {}", name, e),
                )
            })?;

        // Create placeholder Function with signature
        let mut placeholder_func = cranelift_codegen::ir::Function::new();
        placeholder_func.signature = sig.clone();

        // Store placeholder
        self.fns.insert(
            String::from(name),
            GlFunc {
                name: String::from(name),
                clif_sig: sig,
                func_id,
                function: placeholder_func,
            },
        );

        Ok(func_id)
    }

    /// Add a function to fns HashMap without declaring in module
    /// Used for intrinsic functions that are already declared during compilation
    pub fn add_function_to_fns(
        &mut self,
        name: &str,
        sig: cranelift_codegen::ir::Signature,
        func: cranelift_codegen::ir::Function,
        func_id: cranelift_module::FuncId,
    ) {
        self.fns.insert(
            String::from(name),
            GlFunc {
                name: String::from(name),
                clif_sig: sig,
                func_id,
                function: func,
            },
        );
    }

    /// Internal: Get mutable access to Module
    ///
    /// **WARNING**: This is internal-only. Do not use outside of GlModule implementation.
    /// The Module should only be accessed through public builder methods.
    #[doc(hidden)]
    pub(crate) fn module_mut_internal(&mut self) -> &mut M {
        &mut self.module
    }

    /// Internal: Get immutable access to Module (for codegen)
    #[doc(hidden)]
    pub(crate) fn module_internal(&self) -> &M {
        &self.module
    }

    /// Internal helper for apply_transform - contains common logic
    fn apply_transform_impl<T: crate::backend::transform::pipeline::Transform>(
        fns: HashMap<String, GlFunc>,
        transform: T,
        mut new_module: Self,
    ) -> Result<Self, GlslError> {
        use crate::backend::transform::pipeline::TransformContext;
        use cranelift_module::Linkage;

        // 1. Transform all function signatures and create FuncId mappings
        let mut func_id_map = hashbrown::HashMap::new();
        let mut old_func_id_map = hashbrown::HashMap::new();
        for (name, gl_func) in &fns {
            let new_sig = transform.transform_signature(&gl_func.clif_sig);
            // Determine linkage - for now, use Local (can be enhanced later)
            let linkage = Linkage::Local;
            let func_id = new_module
                .module_mut_internal()
                .declare_function(name, linkage, &new_sig)
                .map_err(|e| {
                    GlslError::new(
                        ErrorCode::E0400,
                        format!(
                            "Failed to declare function '{}' in transformed module: {}",
                            name, e
                        ),
                    )
                })?;
            func_id_map.insert(name.clone(), func_id);
            // Build reverse mapping: old FuncId -> function name
            old_func_id_map.insert(gl_func.func_id, name.clone());
        }

        // 1.5. Add builtin function FuncIds to func_id_map
        // Builtins are declared when the module is created, so they should always be available
        {
            use crate::backend::builtins::registry::BuiltinId;
            use cranelift_module::FuncOrDataId;
            for builtin in BuiltinId::all() {
                let name = builtin.name();
                // Get FuncId from module declarations (builtins are declared at module creation)
                if let Some(FuncOrDataId::Func(func_id)) =
                    new_module.module_internal().declarations().get_name(name)
                {
                    func_id_map.insert(alloc::string::String::from(name), func_id);
                }
            }
        }

        // 2. Transform function bodies
        for (name, gl_func) in fns {
            let mut transform_ctx = TransformContext {
                module: &mut new_module,
                func_id_map: func_id_map.clone(),
                old_func_id_map: old_func_id_map.clone(),
            };
            let transformed_func =
                transform.transform_function(&gl_func.function, &mut transform_ctx)?;

            // Use public API to add transformed function
            let new_sig = transform.transform_signature(&gl_func.clif_sig);
            // Determine linkage - for now, use Local (can be enhanced later)
            let linkage = Linkage::Local;
            // Remove the placeholder that was created during declaration
            new_module.fns.remove(&name);
            new_module.add_function(&name, linkage, new_sig, transformed_func)?;
        }

        Ok(new_module)
    }
}

// Specific implementations for each Module type
impl GlModule<JITModule> {
    /// Build executable from JIT module
    /// Returns a boxed GlslExecutable trait object for generic code
    #[allow(unused)]
    pub fn build_executable(
        self,
    ) -> Result<alloc::boxed::Box<dyn crate::exec::executable::GlslExecutable>, GlslError> {
        crate::backend::codegen::jit::build_jit_executable(self).map(|jit| {
            alloc::boxed::Box::new(jit)
                as alloc::boxed::Box<dyn crate::exec::executable::GlslExecutable>
        })
    }

    /// Extract the module (consumes self)
    /// Internal use only - for codegen
    pub(crate) fn into_module(self) -> JITModule {
        self.module
    }

    /// Apply a transform to all functions in this module
    ///
    /// Consumes this GlModule and produces a new GlModule with transformed functions.
    /// Neither module has functions defined (compiled) yet.
    pub fn apply_transform<T: crate::backend::transform::pipeline::Transform>(
        self,
        transform: T,
    ) -> Result<Self, GlslError> {
        let target = self.target.clone();
        let function_registry = self.function_registry;
        let glsl_signatures = self.glsl_signatures;
        let source_text = self.source_text;
        let source_loc_manager = self.source_loc_manager;
        let source_map = self.source_map;
        let fns = self.fns;
        let mut new_module = Self::new_with_target(target)?;
        // Preserve metadata
        new_module.function_registry = function_registry;
        new_module.glsl_signatures = glsl_signatures;
        new_module.source_text = source_text;
        new_module.source_loc_manager = source_loc_manager;
        new_module.source_map = source_map;
        Self::apply_transform_impl(fns, transform, new_module)
    }
}

#[cfg(feature = "emulator")]
impl GlModule<ObjectModule> {
    /// Build executable from Object module (for emulator)
    /// Returns a boxed GlslExecutable trait object for generic code
    #[allow(unused)]
    pub fn build_executable(
        self,
        options: &crate::backend::codegen::emu::EmulatorOptions,
        original_clif: Option<alloc::string::String>,
        transformed_clif: Option<alloc::string::String>,
    ) -> Result<alloc::boxed::Box<dyn crate::exec::executable::GlslExecutable>, GlslError> {
        crate::backend::codegen::emu::build_emu_executable(
            self,
            options,
            original_clif,
            transformed_clif,
        )
        .map(|emu| {
            alloc::boxed::Box::new(emu)
                as alloc::boxed::Box<dyn crate::exec::executable::GlslExecutable>
        })
    }

    /// Extract the module (consumes self)
    /// Internal use only - for codegen
    pub(crate) fn into_module(self) -> ObjectModule {
        self.module
    }

    /// Apply a transform to all functions in this module
    ///
    /// Consumes this GlModule and produces a new GlModule with transformed functions.
    /// Neither module has functions defined (compiled) yet.
    pub fn apply_transform<T: crate::backend::transform::pipeline::Transform>(
        self,
        transform: T,
    ) -> Result<Self, GlslError> {
        let target = self.target.clone();
        let function_registry = self.function_registry;
        let glsl_signatures = self.glsl_signatures;
        let source_text = self.source_text;
        let source_loc_manager = self.source_loc_manager;
        let source_map = self.source_map;
        let fns = self.fns;
        let mut new_module = Self::new_with_target(target)?;
        // Preserve metadata
        new_module.function_registry = function_registry;
        new_module.glsl_signatures = glsl_signatures;
        new_module.source_text = source_text;
        new_module.source_loc_manager = source_loc_manager;
        new_module.source_map = source_map;
        Self::apply_transform_impl(fns, transform, new_module)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "std")]
    fn test_create_jit_module() {
        let target = Target::host_jit().unwrap();
        let gl_module = GlModule::new_jit(target);
        assert!(gl_module.is_ok());
        let gl_module = gl_module.unwrap();
        assert_eq!(gl_module.fns.len(), 0);
    }

    #[test]
    #[cfg(feature = "emulator")]
    fn test_create_object_module() {
        let target = Target::riscv32_emulator().unwrap();
        let gl_module = GlModule::new_object(target);
        assert!(gl_module.is_ok());
        let gl_module = gl_module.unwrap();
        assert_eq!(gl_module.fns.len(), 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_get_func_nonexistent() {
        let target = Target::host_jit().unwrap();
        let gl_module = GlModule::new_jit(target).unwrap();
        assert!(gl_module.get_func("nonexistent").is_none());
    }
}
