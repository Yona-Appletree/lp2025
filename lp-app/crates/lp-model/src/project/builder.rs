//! Project builder for creating test projects with a fluent API

use alloc::{format, string::String};
use serde_json;
use lp_shared::fs::LpFs;
use crate::path::LpPath;
use crate::nodes::fixture::ColorOrder;
use crate::nodes::{
    texture::TextureConfig,
    shader::ShaderConfig,
    output::OutputConfig,
    fixture::FixtureConfig,
    NodeSpecifier,
};

/// Builder for creating test projects
pub struct ProjectBuilder<'a> {
    fs: &'a mut dyn LpFs,
    texture_id: u32,
    shader_id: u32,
    output_id: u32,
    fixture_id: u32,
}

/// Builder for texture nodes
pub struct TextureBuilder {
    width: u32,
    height: u32,
}

/// Builder for shader nodes
pub struct ShaderBuilder {
    texture_path: LpPath,
    glsl_source: String,
    render_order: i32,
}

/// Builder for output nodes
pub struct OutputBuilder {
    pin: u32,
}

/// Builder for fixture nodes
pub struct FixtureBuilder {
    output_path: LpPath,
    texture_path: LpPath,
    mapping: String,
    lamp_type: String,
    color_order: ColorOrder,
    transform: [[f32; 4]; 4],
}

impl<'a> ProjectBuilder<'a> {
    /// Create a new ProjectBuilder
    pub fn new(fs: &'a mut dyn LpFs) -> Self {
        Self {
            fs,
            texture_id: 1,
            shader_id: 1,
            output_id: 1,
            fixture_id: 1,
        }
    }

    /// Set project metadata
    pub fn with_project(mut self, uid: &str, name: &str) -> Self {
        let project_json = format!(r#"{{"uid": "{}", "name": "{}"}}"#, uid, name);
        self.write_file_helper("/project.json", project_json.as_bytes())
            .expect("Failed to write project.json");
        self
    }

    /// Helper to write files - tries write_file_mut for LpFsMemory, falls back to write_file
    fn write_file_helper(&mut self, path: &str, data: &[u8]) -> Result<(), lp_shared::error::FsError> {
        // For LpFsMemory, we need to use write_file_mut
        // Since we can't downcast through trait objects safely, we'll use a workaround:
        // Try write_file first, and if it fails with the specific error, we know it's LpFsMemory
        match self.fs.write_file(path, data) {
            Ok(()) => Ok(()),
            Err(lp_shared::error::FsError::Filesystem(msg)) if msg.contains("write_file_mut") => {
                // This is LpFsMemory - we need mutable access
                // Use unsafe to get mutable access - this is safe because we know it's LpFsMemory
                // and write_file_mut is safe to call
                unsafe {
                    let fs_ptr = self.fs as *mut dyn LpFs;
                    let fs_any = fs_ptr as *mut lp_shared::fs::LpFsMemory;
                    (*fs_any).write_file_mut(path, data)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Start building a texture node
    pub fn texture(&mut self, width: u32, height: u32) -> TextureBuilder {
        TextureBuilder {
            width,
            height,
        }
    }

    /// Start building a shader node
    pub fn shader(&mut self, texture_path: &LpPath) -> ShaderBuilder {
        ShaderBuilder {
            texture_path: texture_path.clone(),
            glsl_source: String::from("vec4 main(vec2 fragCoord, vec2 outputSize, float time) {\n    return vec4(mod(time, 1.0), 0.0, 0.0, 1.0);\n}"),
            render_order: 0,
        }
    }

    /// Start building an output node
    pub fn output(&mut self) -> OutputBuilder {
        OutputBuilder {
            pin: 18,
        }
    }

    /// Start building a fixture node
    pub fn fixture(&mut self, output_path: &LpPath, texture_path: &LpPath) -> FixtureBuilder {
        FixtureBuilder {
            output_path: output_path.clone(),
            texture_path: texture_path.clone(),
            mapping: String::from("linear"),
            lamp_type: String::from("rgb"),
            color_order: ColorOrder::Rgb,
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Build completes - filesystem is already modified
    pub fn build(self) {
        // Filesystem is already modified, nothing to do
    }
}

impl TextureBuilder {
    /// Add the texture node to the project
    pub fn add(self, builder: &mut ProjectBuilder) -> LpPath {
        let id = builder.texture_id;
        builder.texture_id += 1;
        
        let path_str = format!("/src/texture-{}.texture", id);
        let node_path = format!("{}/node.json", path_str);
        
        let config = TextureConfig {
            width: self.width,
            height: self.height,
        };
        
        let json = serde_json::to_string(&config)
            .expect("Failed to serialize texture config");
        
        builder.write_file_helper(&node_path, json.as_bytes())
            .expect("Failed to write texture node.json");
        
        LpPath::from(path_str)
    }
}

impl ShaderBuilder {
    /// Set the GLSL source code
    pub fn glsl(mut self, source: &str) -> Self {
        self.glsl_source = String::from(source);
        self
    }

    /// Set the render order
    pub fn render_order(mut self, order: i32) -> Self {
        self.render_order = order;
        self
    }

    /// Add the shader node to the project
    pub fn add(self, builder: &mut ProjectBuilder) -> LpPath {
        let id = builder.shader_id;
        builder.shader_id += 1;
        
        let path_str = format!("/src/shader-{}.shader", id);
        let node_path = format!("{}/node.json", path_str);
        let glsl_path = format!("{}/main.glsl", path_str);
        
        let config = ShaderConfig {
            glsl_path: String::from("main.glsl"),
            texture_spec: NodeSpecifier::from(self.texture_path.as_str()),
            render_order: self.render_order,
        };
        
        let json = serde_json::to_string(&config)
            .expect("Failed to serialize shader config");
        
        builder.write_file_helper(&node_path, json.as_bytes())
            .expect("Failed to write shader node.json");
        
        builder.write_file_helper(&glsl_path, self.glsl_source.as_bytes())
            .expect("Failed to write shader GLSL file");
        
        LpPath::from(path_str)
    }
}

impl OutputBuilder {
    /// Set the GPIO pin
    pub fn gpio_pin(mut self, pin: u32) -> Self {
        self.pin = pin;
        self
    }

    /// Add the output node to the project
    pub fn add(self, builder: &mut ProjectBuilder) -> LpPath {
        let id = builder.output_id;
        builder.output_id += 1;
        
        let path_str = format!("/src/output-{}.output", id);
        let node_path = format!("{}/node.json", path_str);
        
        let config = OutputConfig::GpioStrip { pin: self.pin };
        
        let json = serde_json::to_string(&config)
            .expect("Failed to serialize output config");
        
        builder.write_file_helper(&node_path, json.as_bytes())
            .expect("Failed to write output node.json");
        
        LpPath::from(path_str)
    }
}

impl FixtureBuilder {
    /// Set the mapping type
    pub fn mapping(mut self, mapping: &str) -> Self {
        self.mapping = String::from(mapping);
        self
    }

    /// Set the lamp type
    pub fn lamp_type(mut self, lamp_type: &str) -> Self {
        self.lamp_type = String::from(lamp_type);
        self
    }

    /// Set the color order
    pub fn color_order(mut self, order: ColorOrder) -> Self {
        self.color_order = order;
        self
    }

    /// Set the transform matrix
    pub fn transform(mut self, transform: [[f32; 4]; 4]) -> Self {
        self.transform = transform;
        self
    }

    /// Add the fixture node to the project
    pub fn add(self, builder: &mut ProjectBuilder) -> LpPath {
        let id = builder.fixture_id;
        builder.fixture_id += 1;
        
        let path_str = format!("/src/fixture-{}.fixture", id);
        let node_path = format!("{}/node.json", path_str);
        
        let config = FixtureConfig {
            output_spec: NodeSpecifier::from(self.output_path.as_str()),
            texture_spec: NodeSpecifier::from(self.texture_path.as_str()),
            mapping: self.mapping,
            lamp_type: self.lamp_type,
            color_order: self.color_order,
            transform: self.transform,
        };
        
        let json = serde_json::to_string(&config)
            .expect("Failed to serialize fixture config");
        
        builder.write_file_helper(&node_path, json.as_bytes())
            .expect("Failed to write fixture node.json");
        
        LpPath::from(path_str)
    }
}
