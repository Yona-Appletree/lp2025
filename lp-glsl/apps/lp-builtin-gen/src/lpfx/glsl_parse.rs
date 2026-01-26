//! Parse GLSL signature strings into FunctionSignature

use glsl::parser::Parse;
use glsl::syntax::ExternalDeclaration;
use lp_glsl_compiler::frontend::semantic::functions::FunctionSignature;
use lp_glsl_compiler::frontend::semantic::passes::function_signature::extract_function_signature;
use crate::lpfx::errors::LpfxCodegenError;

/// Parse a GLSL function signature string into a FunctionSignature
pub fn parse_glsl_signature(
    sig_str: &str,
    function_name: &str,
    file_path: &str,
) -> Result<FunctionSignature, LpfxCodegenError> {
    // Wrap the signature in a function call to make it parseable
    // We'll parse: void wrapper() { func(); }
    // Where func is the signature we want to parse
    let wrapper = format!("void wrapper() {{ {}(); }}", sig_str);
    
    // Parse the GLSL code
    let shader = Parse::parse(&wrapper).map_err(|e| {
        // Extract a cleaner error message
        let error_msg = e
            .info
            .lines()
            .find(|line| {
                let trimmed = line.trim();
                trimmed.contains("expected") || trimmed.contains("found")
            })
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| format!("GLSL parse error: {}", e));
        
        LpfxCodegenError::InvalidSignature {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            signature: sig_str.to_string(),
            error: error_msg,
        }
    })?;
    
    // Extract the function prototype from the parsed shader
    // The shader should have one external declaration which is our wrapper function
    // Inside that wrapper, there should be a call to our function
    // We need to extract the function prototype from the call
    
    // Actually, we need a different approach - we want to parse the function signature itself
    // Let's try parsing it as a function prototype directly
    let prototype_str = format!("{};", sig_str);
    let shader = Parse::parse(&prototype_str).map_err(|e| {
        let error_msg = e
            .info
            .lines()
            .find(|line| {
                let trimmed = line.trim();
                trimmed.contains("expected") || trimmed.contains("found")
            })
            .map(|line| line.trim().to_string())
            .unwrap_or_else(|| format!("GLSL parse error: {}", e));
        
        LpfxCodegenError::InvalidSignature {
            function_name: function_name.to_string(),
            file_path: file_path.to_string(),
            signature: sig_str.to_string(),
            error: error_msg,
        }
    })?;
    
    // Find the function prototype in the shader
    for decl in &shader.0 {
        if let ExternalDeclaration::FunctionPrototype(prototype) = decl {
            return extract_function_signature(prototype).map_err(|e| {
                LpfxCodegenError::InvalidSignature {
                    function_name: function_name.to_string(),
                    file_path: file_path.to_string(),
                    signature: sig_str.to_string(),
                    error: format!("Failed to extract function signature: {}", e),
                }
            });
        }
    }
    
    Err(LpfxCodegenError::InvalidSignature {
        function_name: function_name.to_string(),
        file_path: file_path.to_string(),
        signature: sig_str.to_string(),
        error: "No function prototype found in parsed GLSL".to_string(),
    })
}
