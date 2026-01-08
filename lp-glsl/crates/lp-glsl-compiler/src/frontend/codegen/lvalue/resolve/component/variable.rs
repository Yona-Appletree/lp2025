//! Component access on Variable LValue

use crate::semantic::types::Type as GlslType;
use alloc::vec::Vec;
use cranelift_frontend::Variable;

use super::super::super::types::LValue;

/// Resolve component access on a Variable LValue
pub fn resolve_component_on_variable(
    vars: Vec<Variable>,
    base_ty: GlslType,
    indices: Vec<usize>,
    result_ty: GlslType,
) -> LValue {
    LValue::Component {
        base_vars: vars,
        base_ty,
        indices,
        result_ty,
    }
}
