//! Code generation for LPFX Functions
//!
//! Maps user-facing `lpfx_*` function names to internal implementations
//! and handles vector argument flattening.

use crate::DecimalFormat;
use crate::error::{ErrorCode, GlslError};
use crate::frontend::codegen::context::CodegenContext;
use crate::frontend::semantic::lpfx::lpfx_fn_registry::find_lpfx_fn;
use crate::frontend::semantic::lpfx::lpfx_sig::{build_call_signature, expand_vector_args};
use crate::semantic::types::Type;
use alloc::{format, vec, vec::Vec};
use cranelift_codegen::ir::{ExtFuncData, ExternalName, FuncRef, InstBuilder, Value};

impl<'a, M: cranelift_module::Module> CodegenContext<'a, M> {
    /// Emit code for an LPFX function call.
    ///
    /// # Arguments
    /// * `name` - Function name (e.g., "lpfx_hash1", "lpfx_snoise2")
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
            GlslError::new(ErrorCode::E0400, format!("Unknown LPFX function: {name}"))
        })?;

        // Collect parameter types before flattening (needed for signature)
        let param_types: Vec<Type> = args.iter().map(|(_, ty)| ty.clone()).collect();

        // Flatten vector arguments to individual components
        let mut flat_values = Vec::new();
        let mut flat_types = Vec::new(); // Track types for each flattened value
        for ((vals, _ty), param_ty) in args.iter().zip(&param_types) {
            let base_ty = if param_ty.is_vector() {
                param_ty.vector_base_type().unwrap()
            } else {
                param_ty.clone()
            };
            for _val in vals {
                flat_types.push(base_ty.clone());
            }
            flat_values.extend(vals);
        }
        let flat_args = expand_vector_args(&param_types, &flat_values);

        // Handle Decimal vs NonDecimal implementations
        match &func.impls {
            crate::frontend::semantic::lpfx::lpfx_fn::LpfxFnImpl::Decimal {
                float_impl, ..
            } => {
                // Always use float implementation in frontend - transform will convert to q32
                // Generate TestCase call with float signature (f32 args, f32 return)
                let func_ref = self.get_lpfx_testcase_call(func, *float_impl, &param_types)?;

                // Emit call instruction with float arguments (no conversion needed)
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

                // Return float result directly (no conversion needed)
                Ok((vec![results[0]], func.glsl_sig.return_type.clone()))
            }
            crate::frontend::semantic::lpfx::lpfx_fn::LpfxFnImpl::NonDecimal(builtin_id) => {
                // Direct builtin call (hash functions don't need conversion)
                let func_ref = self
                    .gl_module
                    .get_builtin_func_ref(*builtin_id, self.builder.func)?;

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
    }

    /// Helper to declare and get FuncRef for LPFX function TestCase call.
    ///
    /// Creates external function calls using TestCase names (e.g., "__lpfx_snoise3").
    /// These are converted to q32 builtins by the transform.
    ///
    /// Always uses float signature (f32 args, f32 return) - the transform will handle
    /// conversion to q32 when processing the TestCase call.
    fn get_lpfx_testcase_call(
        &mut self,
        func: &'static crate::frontend::semantic::lpfx::lpfx_fn::LpfxFn,
        builtin_id: crate::backend::builtins::registry::BuiltinId,
        _param_types: &[Type],
    ) -> Result<FuncRef, GlslError> {
        // TestCase name is the GLSL function name with __ prefix
        let testcase_name = format!("__{}", func.glsl_sig.name);

        // Get pointer type for StructReturn (if needed)
        let pointer_type = self.gl_module.module_internal().isa().pointer_type();

        // Build signature with Float format (f32 args, f32 return)
        // The transform will convert this to q32 when processing the call
        let sig = build_call_signature(func, builtin_id, DecimalFormat::Float, pointer_type);

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
