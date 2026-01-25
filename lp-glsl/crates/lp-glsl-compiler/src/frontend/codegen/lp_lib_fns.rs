//! Code generation for LPFX Functions
//!
//! Maps user-facing `lpfx_*` function names to internal implementations
//! and handles vector argument flattening.

use crate::DecimalFormat;
use crate::error::{ErrorCode, GlslError};
use crate::frontend::codegen::context::CodegenContext;
use crate::frontend::semantic::lpfx::lpfx_fn_registry::{
    find_lpfx_fn, get_impl_for_format, rust_fn_name_to_builtin_id,
};
use crate::frontend::semantic::lpfx::lpfx_sig::{build_call_signature, expand_vector_args};
use crate::semantic::types::Type;
use alloc::{format, vec, vec::Vec};
use cranelift_codegen::ir::{ExtFuncData, ExternalName, FuncRef, InstBuilder, Value};

impl<'a, M: cranelift_module::Module> CodegenContext<'a, M> {
    /// Emit code for an LPFX function call.
    ///
    /// # Arguments
    /// * `name` - Function name (e.g., "lpfx_hash1", "lpfx_simplex2")
    /// * `args` - Vector of (value, type) pairs for each argument
    ///
    /// # Returns
    /// Tuple of (result_values, return_type)
    pub fn emit_lp_lib_fn_call(
        &mut self,
        name: &str,
        args: Vec<(Vec<Value>, Type)>,
    ) -> Result<(Vec<Value>, Type), GlslError> {
        // Look up function in registry
        let func = find_lpfx_fn(name).ok_or_else(|| {
            GlslError::new(ErrorCode::E0400, format!("Unknown LPFX function: {}", name))
        })?;

        // Collect parameter types before flattening (needed for signature)
        let param_types: Vec<Type> = args.iter().map(|(_, ty)| ty.clone()).collect();

        // Flatten vector arguments to individual components
        let mut flat_values = Vec::new();
        for (vals, ty) in args {
            flat_values.extend(vals);
        }
        let flat_args = expand_vector_args(&param_types, &flat_values);

        // Determine decimal format - use Fixed32 for functions that need it, otherwise doesn't matter
        // (hash functions don't use decimal formats)
        let format = if func
            .impls
            .iter()
            .any(|impl_| impl_.decimal_format.is_some())
        {
            DecimalFormat::Fixed32
        } else {
            DecimalFormat::Fixed32 // Default, but won't be used for hash functions
        };

        // Get implementation for this format
        let impl_ = get_impl_for_format(func, format).ok_or_else(|| {
            GlslError::new(
                ErrorCode::E0400,
                format!(
                    "No implementation found for LPFX function {} with format {:?}",
                    name, format
                ),
            )
        })?;

        // Check if this function needs fixed32 mapping (has decimal_format)
        if impl_.decimal_format.is_some() {
            // Emit TestCase call - transform will convert to fixed32 builtin
            let func_ref = self.get_lpfx_testcase_call(func, impl_, &param_types, format)?;

            // Emit call instruction
            let call_inst = self.builder.ins().call(func_ref, &flat_args);

            // Extract return value(s)
            let results = self.builder.inst_results(call_inst);
            if results.len() != 1 {
                return Err(GlslError::new(
                    ErrorCode::E0400,
                    format!(
                        "Expected 1 return value from LPFX function, got {}",
                        results.len()
                    ),
                ));
            }

            Ok((vec![results[0]], func.glsl_sig.return_type.clone()))
        } else {
            // Direct builtin call (hash functions don't need conversion)
            let builtin_id = rust_fn_name_to_builtin_id(impl_.rust_fn_name).ok_or_else(|| {
                GlslError::new(
                    ErrorCode::E0400,
                    format!("Unknown builtin for function: {}", impl_.rust_fn_name),
                )
            })?;

            let func_ref = self
                .gl_module
                .get_builtin_func_ref(builtin_id, self.builder.func)?;

            // Emit call instruction
            let call_inst = self.builder.ins().call(func_ref, &flat_args);

            // Extract return value(s)
            let results = self.builder.inst_results(call_inst);
            if results.len() != 1 {
                return Err(GlslError::new(
                    ErrorCode::E0400,
                    format!(
                        "Expected 1 return value from LPFX function, got {}",
                        results.len()
                    ),
                ));
            }

            Ok((vec![results[0]], func.glsl_sig.return_type.clone()))
        }
    }

    /// Helper to declare and get FuncRef for LPFX function TestCase call.
    ///
    /// Creates external function calls using TestCase names (e.g., "__lpfx_simplex3").
    /// These are converted to fixed32 builtins by the transform.
    fn get_lpfx_testcase_call(
        &mut self,
        func: &crate::frontend::semantic::lpfx::lpfx_fn::LpfxFn,
        impl_: &crate::frontend::semantic::lpfx::lpfx_fn::LpfxFnImpl,
        param_types: &[Type],
        format: DecimalFormat,
    ) -> Result<FuncRef, GlslError> {
        let testcase_name = impl_.rust_fn_name;

        // Build signature dynamically
        let sig = build_call_signature(func, impl_, format);

        // Create TestCase name for external function call
        let sig_ref = self.builder.func.import_signature(sig);
        let ext_name = ExternalName::testcase(testcase_name.as_bytes());
        let ext_func = ExtFuncData {
            name: ext_name,
            signature: sig_ref,
            colocated: false,
        };
        Ok(self.builder.func.import_function(ext_func))
    }
}
