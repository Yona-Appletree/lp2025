//! Shader node runtime

use crate::error::Error;
use crate::nodes::id::TextureId;
use crate::nodes::shader::config::ShaderNode;
use crate::project::runtime::NodeStatus;
use crate::runtime::contexts::ShaderRenderContext;
use crate::runtime::lifecycle::NodeLifecycle;
use alloc::{
    format,
    string::{String, ToString},
    vec,
};
use lp_glsl_compiler::frontend::semantic::types::Type;
use lp_glsl_compiler::{DecimalFormat, GlslExecutable, GlslOptions, GlslValue, RunMode, glsl_jit};

/// Shader node runtime
pub struct ShaderNodeRuntime {
    config: ShaderNode,
    executable: Option<alloc::boxed::Box<dyn GlslExecutable>>,
    texture_id: TextureId,
    status: NodeStatus,
}

impl ShaderNodeRuntime {
    /// Create a new shader node runtime (uninitialized)
    pub fn new() -> Self {
        Self {
            config: ShaderNode::Single {
                glsl: String::new(),
                texture_id: TextureId(String::new()),
            }, // Temporary, will be replaced in init
            executable: None,
            texture_id: TextureId(String::new()),
            status: NodeStatus::Ok,
        }
    }

    /// Get the current status
    pub fn status(&self) -> &NodeStatus {
        &self.status
    }

    /// Get the texture ID this shader writes to
    pub fn texture_id(&self) -> TextureId {
        self.texture_id.clone()
    }

    /// Get the shader configuration
    pub fn config(&self) -> &ShaderNode {
        &self.config
    }
}

impl Default for ShaderNodeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeLifecycle for ShaderNodeRuntime {
    type Config = ShaderNode;
    type RenderContext<'a> = ShaderRenderContext<'a>;

    fn init(
        &mut self,
        config: &Self::Config,
        _ctx: &crate::runtime::contexts::InitContext,
    ) -> Result<(), Error> {
        // Store config
        self.config = config.clone();

        match config {
            ShaderNode::Single { glsl, texture_id } => {
                self.texture_id = texture_id.clone();

                // Create compilation options
                // Use Fixed32 format - Float format is not yet supported (causes TestCase relocation errors)
                let options = GlslOptions {
                    run_mode: RunMode::HostJit,
                    decimal_format: DecimalFormat::Fixed32,
                };

                // Compile GLSL
                match glsl_jit(glsl, options) {
                    Ok(executable) => {
                        // Validate signature: vec4 main(vec2 fragCoord, vec2 outputSize, float time)
                        if let Some(sig) = executable.get_function_signature("main") {
                            // Check return type is vec4
                            let is_vec4 = matches!(sig.return_type, Type::Vec4);
                            if !is_vec4 {
                                self.status = NodeStatus::Error {
                                    status_message: format!(
                                        "Shader main() must return vec4, got {:?}",
                                        sig.return_type
                                    ),
                                };
                                return Err(Error::Validation(format!(
                                    "Shader main() must return vec4, got {:?}",
                                    sig.return_type
                                )));
                            }

                            // Check parameters: vec2 fragCoord, vec2 outputSize, float time
                            if sig.parameters.len() != 3 {
                                self.status = NodeStatus::Error {
                                    status_message: format!(
                                        "Shader main() must have 3 parameters (vec2 fragCoord, vec2 outputSize, float time), got {}",
                                        sig.parameters.len()
                                    ),
                                };
                                return Err(Error::Validation(format!(
                                    "Shader main() must have 3 parameters, got {}",
                                    sig.parameters.len()
                                )));
                            }

                            // Check parameter types (simplified - just check count for now)
                            // Full validation would check each parameter type
                            self.executable = Some(executable);
                            self.status = NodeStatus::Ok;
                            Ok(())
                        } else {
                            self.status = NodeStatus::Error {
                                status_message: "Shader must have a main() function".to_string(),
                            };
                            Err(Error::Validation(
                                "Shader must have a main() function".to_string(),
                            ))
                        }
                    }
                    Err(e) => {
                        // Format the error message - GlslError Display already includes
                        // nice formatting with line numbers and carets if span_text is set
                        let error_msg = format!("{}", e);
                        self.status = NodeStatus::Error {
                            status_message: format!("Shader compilation failed: {}", error_msg),
                        };
                        // Preserve the formatted error message (with newlines for span_text)
                        Err(Error::Node(format!(
                            "Shader compilation failed: {}",
                            error_msg
                        )))
                    }
                }
            }
        }
    }

    fn render(&mut self, ctx: &mut Self::RenderContext<'_>) -> Result<(), Error> {
        // Skip if executable is None (compilation failed)
        let executable = match &mut self.executable {
            Some(exec) => exec,
            None => return Ok(()),
        };

        // Read time before borrowing texture
        let time_seconds = ctx.time.total_ms as f32 / 1000.0;

        // Get texture to write to
        let texture = match ctx.get_texture_mut(self.texture_id.clone()) {
            Some(tex) => tex,
            None => {
                self.status = NodeStatus::Error {
                    status_message: format!(
                        "Texture {} not found",
                        String::from(self.texture_id.clone())
                    ),
                };
                return Err(Error::Node(format!(
                    "Texture {} not found",
                    String::from(self.texture_id.clone())
                )));
            }
        };

        let width = texture.width();
        let height = texture.height();

        // Iterate over all texture pixels
        for y in 0..height {
            for x in 0..width {
                // Convert pixel coordinates to fragCoord (vec2)
                let frag_coord = [
                    x as f32 + 0.5, // Center of pixel
                    y as f32 + 0.5,
                ];

                // Output size (vec2)
                let output_size = [width as f32, height as f32];

                // Call shader: vec4 main(vec2 fragCoord, vec2 outputSize, float time)
                let args = vec![
                    GlslValue::Vec2(frag_coord),
                    GlslValue::Vec2(output_size),
                    GlslValue::F32(time_seconds),
                ];

                match executable.call_vec("main", &args, 4) {
                    Ok(result) => {
                        // result is Vec<f32> with 4 elements [r, g, b, a]
                        // Convert to [u8; 4] (clamp to [0, 1] and scale to [0, 255])
                        let r = (result[0].max(0.0).min(1.0) * 255.0) as u8;
                        let g = (result[1].max(0.0).min(1.0) * 255.0) as u8;
                        let b = (result[2].max(0.0).min(1.0) * 255.0) as u8;
                        let a = (result[3].max(0.0).min(1.0) * 255.0) as u8;

                        texture.set_pixel(x, y, [r, g, b, a]);
                    }
                    Err(e) => {
                        self.status = NodeStatus::Error {
                            status_message: format!("Shader execution failed: {}", e),
                        };
                        return Err(Error::Node(format!("Shader execution failed: {}", e)));
                    }
                }
            }
        }

        Ok(())
    }

    fn destroy(&mut self) -> Result<(), Error> {
        // Cleanup executable if needed
        self.executable = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::texture::TextureNode;
    use hashbrown::HashMap;

    #[test]
    fn test_shader_node_runtime_init_valid() {
        let mut runtime = ShaderNodeRuntime::new();
        let glsl = r#"
vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    return vec4(0.5, 0.5, 0.5, 1.0);
}
"#;
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new()
            .with_uid("test".to_string())
            .with_name("Test".to_string())
            .add_texture(crate::nodes::texture::TextureNode::Memory {
                size: [64, 64],
                format: crate::nodes::texture::formats::RGBA8.to_string(),
            });
        let config = ShaderNode::Single {
            glsl: glsl.to_string(),
            texture_id,
        };
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );

        assert!(runtime.init(&config, &ctx).is_ok());
        assert!(runtime.executable.is_some());
        assert!(matches!(runtime.status(), NodeStatus::Ok));
    }

    #[test]
    fn test_shader_node_runtime_init_invalid_glsl() {
        let mut runtime = ShaderNodeRuntime::new();
        let glsl = "invalid glsl code";
        let builder = crate::project::builder::ProjectBuilder::new_test();
        let (builder, texture_id) = builder.add_texture(TextureNode::Memory {
            size: [64, 64],
            format: crate::nodes::texture::formats::RGB8.to_string(),
        });
        let config = ShaderNode::Single {
            glsl: glsl.to_string(),
            texture_id,
        };
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );

        assert!(runtime.init(&config, &ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_shader_node_runtime_update_skips_if_no_executable() {
        let mut runtime = ShaderNodeRuntime::new();
        runtime.executable = None;

        let frame_time = crate::runtime::frame_time::FrameTime::new(16, 1000);
        let mut textures: HashMap<TextureId, crate::nodes::texture::TextureNodeRuntime> =
            HashMap::new();
        let mut ctx = ShaderRenderContext::new(frame_time, &mut textures);

        // Should return Ok without error
        assert!(runtime.render(&mut ctx).is_ok());
    }

    #[test]
    fn test_shader_node_runtime_init_wrong_return_type() {
        let mut runtime = ShaderNodeRuntime::new();
        let glsl = r#"
vec3 main(vec2 fragCoord, vec2 outputSize, float time) {
    return vec3(1.0, 1.0, 1.0);
}
"#;
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(crate::nodes::texture::TextureNode::Memory {
                size: [64, 64],
                format: crate::nodes::texture::formats::RGBA8.to_string(),
            });
        let config = ShaderNode::Single {
            glsl: glsl.to_string(),
            texture_id,
        };
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );

        assert!(runtime.init(&config, &ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_shader_node_runtime_init_wrong_parameter_count() {
        let mut runtime = ShaderNodeRuntime::new();
        let glsl = r#"
vec4 main(vec2 fragCoord, float time) {
    return vec4(1.0, 1.0, 1.0, 1.0);
}
"#;
        let (builder, texture_id) = crate::project::builder::ProjectBuilder::new_test()
            .add_texture(crate::nodes::texture::TextureNode::Memory {
                size: [64, 64],
                format: crate::nodes::texture::formats::RGBA8.to_string(),
            });
        let config = ShaderNode::Single {
            glsl: glsl.to_string(),
            texture_id,
        };
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );

        assert!(runtime.init(&config, &ctx).is_err());
        assert!(matches!(runtime.status(), NodeStatus::Error { .. }));
    }

    #[test]
    fn test_shader_node_runtime_update_executes_shader_and_writes_pixels() {
        // Create texture runtime
        let mut texture_runtime = crate::nodes::texture::TextureNodeRuntime::new();
        let texture_config = crate::nodes::texture::TextureNode::Memory {
            size: [4, 4],
            format: crate::nodes::texture::formats::RGBA8.to_string(),
        };
        let (builder, texture_id) =
            crate::project::builder::ProjectBuilder::new_test().add_texture(texture_config.clone());
        let (textures, shaders, outputs, fixtures) = builder.node_maps();
        let project_config = builder.build().unwrap();
        let init_ctx = crate::runtime::contexts::InitContext::new(
            &project_config,
            &textures,
            &shaders,
            &outputs,
            &fixtures,
        );
        texture_runtime.init(&texture_config, &init_ctx).unwrap();

        // Create shader runtime
        let mut shader_runtime = ShaderNodeRuntime::new();
        let glsl = r#"
vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    // Return a simple color based on position
    return vec4(0.5, 0.5, 0.5, 1.0);
}
"#;
        let shader_config = ShaderNode::Single {
            glsl: glsl.to_string(),
            texture_id,
        };
        shader_runtime.init(&shader_config, &init_ctx).unwrap();

        // Verify texture is initially zero
        let pixel_before = texture_runtime.texture().get_pixel(0, 0).unwrap();
        assert_eq!(pixel_before, [0, 0, 0, 0]);

        // Create render context and call update
        let frame_time = crate::runtime::frame_time::FrameTime::new(16, 1000);
        let mut textures: HashMap<TextureId, crate::nodes::texture::TextureNodeRuntime> =
            HashMap::new();
        textures.insert(texture_id, texture_runtime);
        let mut ctx = ShaderRenderContext::new(frame_time, &mut textures);

        assert!(shader_runtime.render(&mut ctx).is_ok());

        // Verify pixels were written
        let texture = ctx.textures.get(&texture_id).unwrap();

        // Check that at least some pixels are non-zero (shader executed)
        let mut found_non_zero = false;
        for y in 0..4 {
            for x in 0..4 {
                let pixel = texture.texture().get_pixel(x, y).unwrap();
                if pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0 {
                    found_non_zero = true;
                    break;
                }
            }
            if found_non_zero {
                break;
            }
        }
        assert!(found_non_zero, "Shader should have written non-zero pixels");

        // Check a specific pixel - at (0,0) with 0.5 offset, fragCoord is [0.5, 0.5]
        // outputSize is [4.0, 4.0], so normalized is [0.125, 0.125]
        // Expected: [0.125 * 255, 0.125 * 255, 0.5 * 255, 1.0 * 255] ≈ [32, 32, 128, 255]
        let pixel_00 = texture.texture().get_pixel(0, 0).unwrap();
        // Alpha should always be 255
        assert_eq!(pixel_00[3], 255);
        // Blue channel should be around 128 (0.5 * 255)
        assert!(
            pixel_00[2] >= 120 && pixel_00[2] <= 135,
            "Blue channel should be ~128, got {}",
            pixel_00[2]
        );

        // Check that all pixels have alpha = 255
        for y in 0..4 {
            for x in 0..4 {
                let pixel = texture.texture().get_pixel(x, y).unwrap();
                assert_eq!(pixel[3], 255, "All pixels should have alpha = 255");
            }
        }
    }
}
