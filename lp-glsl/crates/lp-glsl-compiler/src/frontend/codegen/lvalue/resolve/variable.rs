//! Variable expression resolution

use crate::error::{GlslError, extract_span_from_identifier, source_span_to_location};
use crate::frontend::codegen::context::CodegenContext;
use alloc::vec::Vec;

use super::super::types::LValue;

/// Resolve a variable expression to an LValue
pub fn resolve_variable_lvalue<M: cranelift_module::Module>(
    ctx: &mut CodegenContext<'_, M>,
    ident: &glsl::syntax::Identifier,
) -> Result<LValue, GlslError> {
    let span = extract_span_from_identifier(ident);

    // Get variable type first to check if it's an array
    let ty = ctx
        .lookup_variable_type(&ident.name)
        .ok_or_else(|| {
            let error = GlslError::undefined_variable(&ident.name)
                .with_location(source_span_to_location(&span));
            ctx.add_span_to_error(error, &span)
        })?
        .clone();

    // For arrays, return LValue::Variable with empty vars (arrays use pointer-based storage)
    if ty.is_array() {
        return Ok(LValue::Variable {
            vars: Vec::new(),
            ty,
        });
    }

    let vars = ctx
        .lookup_variables(&ident.name)
        .ok_or_else(|| {
            let error = GlslError::undefined_variable(&ident.name)
                .with_location(source_span_to_location(&span));
            ctx.add_span_to_error(error, &span)
        })?
        .to_vec();

    Ok(LValue::Variable { vars, ty })
}
