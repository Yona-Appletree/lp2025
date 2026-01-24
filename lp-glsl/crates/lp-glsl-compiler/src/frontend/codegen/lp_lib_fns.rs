//! Code generation for LP Library Functions
//!
//! Maps user-facing `lp_*` function names to internal `BuiltinId` variants
//! and handles vector argument flattening.

use crate::error::{ErrorCode, GlslError};
use crate::frontend::codegen::context::CodegenContext;
use crate::frontend::semantic::lp_lib_fns::LpLibFn;
use crate::frontend::semantic::types::Type;
use cranelift_codegen::ir::{
    AbiParam, ExtFuncData, ExternalName, FuncRef, InstBuilder, Signature, Value, types,
};
use cranelift_codegen::isa::CallConv;

use alloc::{format, vec, vec::Vec};

impl<'a, M: cranelift_module::Module> CodegenContext<'a, M> {
    /// Emit code for an LP library function call.
    ///
    /// # Arguments
    /// * `name` - Function name (e.g., "lp_hash", "lp_simplex2")
    /// * `args` - Vector of (value, type) pairs for each argument
    ///
    /// # Returns
    /// Tuple of (result_values, return_type)
    pub fn emit_lp_lib_fn_call(
        &mut self,
        name: &str,
        args: Vec<(Vec<Value>, Type)>,
    ) -> Result<(Vec<Value>, Type), GlslError> {
        // Determine which BuiltinId to use based on name and argument count
        let lp_fn = LpLibFn::from_name_and_args(name, args.len()).ok_or_else(|| {
            GlslError::new(
                ErrorCode::E0400,
                format!(
                    "Unknown LP library function: {} with {} arguments",
                    name,
                    args.len()
                ),
            )
        })?;
        let builtin_id = lp_fn.builtin_id();

        // Flatten vector arguments to individual components
        let mut flat_args = Vec::new();
        for (vals, ty) in args {
            match ty {
                Type::Vec2 | Type::IVec2 | Type::UVec2 => {
                    // Extract x and y components
                    if vals.len() != 2 {
                        return Err(GlslError::new(
                            ErrorCode::E0400,
                            format!("Expected 2 values for vec2 argument, got {}", vals.len()),
                        ));
                    }
                    flat_args.push(vals[0]);
                    flat_args.push(vals[1]);
                }
                Type::Vec3 | Type::IVec3 | Type::UVec3 => {
                    // Extract x, y, and z components
                    if vals.len() != 3 {
                        return Err(GlslError::new(
                            ErrorCode::E0400,
                            format!("Expected 3 values for vec3 argument, got {}", vals.len()),
                        ));
                    }
                    flat_args.push(vals[0]);
                    flat_args.push(vals[1]);
                    flat_args.push(vals[2]);
                }
                Type::Float | Type::Int | Type::UInt => {
                    // Scalar argument - single value
                    if vals.len() != 1 {
                        return Err(GlslError::new(
                            ErrorCode::E0400,
                            format!("Expected 1 value for scalar argument, got {}", vals.len()),
                        ));
                    }
                    flat_args.push(vals[0]);
                }
                _ => {
                    return Err(GlslError::new(
                        ErrorCode::E0400,
                        format!("Unsupported argument type for LP library function: {ty:?}",),
                    ));
                }
            }
        }

        // Check if this function needs fixed32 mapping
        if lp_fn.needs_fixed32_mapping() {
            // Emit TestCase call - transform will convert to fixed32 builtin
            let func_ref = self.get_lp_lib_testcase_call(&lp_fn, flat_args.len())?;

            // Emit call instruction
            let call_inst = self.builder.ins().call(func_ref, &flat_args);

            // Extract return value(s)
            let results = self.builder.inst_results(call_inst);
            if results.len() != 1 {
                return Err(GlslError::new(
                    ErrorCode::E0400,
                    format!(
                        "Expected 1 return value from LP library function, got {}",
                        results.len()
                    ),
                ));
            }

            // Get return type from the enum
            let return_type = lp_fn.return_type();

            Ok((vec![results[0]], return_type))
        } else {
            // Direct builtin call (hash functions don't need conversion)
            let func_ref = self
                .gl_module
                .get_builtin_func_ref(builtin_id, self.builder.func)?;

            // Build call arguments
            let call_args: Vec<Value> = flat_args;

            // Emit call instruction
            let call_inst = self.builder.ins().call(func_ref, &call_args);

            // Extract return value(s)
            let results = self.builder.inst_results(call_inst);
            if results.len() != 1 {
                return Err(GlslError::new(
                    ErrorCode::E0400,
                    format!(
                        "Expected 1 return value from LP library function, got {}",
                        results.len()
                    ),
                ));
            }

            // Get return type from the enum
            let return_type = lp_fn.return_type();

            Ok((vec![results[0]], return_type))
        }
    }

    /// Helper to declare and get FuncRef for LP library function TestCase call.
    ///
    /// Creates external function calls using TestCase names (e.g., "__lp_simplex3").
    /// These are converted to fixed32 builtins by the transform.
    fn get_lp_lib_testcase_call(
        &mut self,
        lp_fn: &LpLibFn,
        arg_count: usize,
    ) -> Result<FuncRef, GlslError> {
        let testcase_name = lp_fn.symbol_name();

        // Create signature with F32 types (before transform)
        let mut sig = Signature::new(CallConv::SystemV);
        for _ in 0..arg_count {
            sig.params.push(AbiParam::new(types::F32));
        }
        sig.returns.push(AbiParam::new(types::F32));

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
