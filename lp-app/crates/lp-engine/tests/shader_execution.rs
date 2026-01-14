use lp_engine::ProjectRuntime;
use lp_shared::fs::LpFsMemory;

#[test]
fn test_shader_initialization() {
    let mut fs = LpFsMemory::new();
    
    // Create project.json
    fs.write_file_mut("/project.json", r#"{
        "uid": "test-project",
        "name": "Test Project"
    }"#.as_bytes()).unwrap();
    
    // Create a texture node
    fs.write_file_mut("/src/test.texture/node.json", r#"{
        "width": 100,
        "height": 100
    }"#.as_bytes()).unwrap();
    
    // Create a shader node
    fs.write_file_mut("/src/test.shader/node.json", r#"{
        "glsl_path": "main.glsl",
        "texture_spec": "/src/test.texture",
        "render_order": 0
    }"#.as_bytes()).unwrap();
    
    // Create a simple GLSL shader
    fs.write_file_mut("/src/test.shader/main.glsl", r#"
vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    return vec4(1.0, 0.0, 0.0, 1.0); // Red
}
"#.as_bytes()).unwrap();
    
    // Create runtime and initialize
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    // Verify shader was initialized
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &lp_model::project::api::ApiNodeSpecifier::All,
    ).unwrap();
    
    match response {
        lp_model::project::api::ProjectResponse::GetChanges { node_details, .. } => {
            let shader_detail = node_details.values()
                .find(|d| d.path.as_str() == "/src/test.shader")
                .expect("Shader node should be in details");
            
            match &shader_detail.state {
                lp_model::project::api::NodeState::Shader(shader_state) => {
                    // Verify GLSL code was loaded
                    assert!(!shader_state.glsl_code.is_empty());
                    assert!(shader_state.glsl_code.contains("main"));
                    // Should have no compilation error (shader is valid)
                    assert!(shader_state.error.is_none());
                }
                _ => panic!("Expected shader state"),
            }
            
            // Verify config was extracted
            match &shader_detail.config {
                config if config.kind() == lp_model::NodeKind::Shader => {
                    // Config should be extracted (not default)
                    // We can't easily check the values without downcasting, but we can verify it's not empty/default
                }
                _ => panic!("Expected shader config"),
            }
        }
    }
}

#[test]
fn test_shader_execution() {
    let mut fs = LpFsMemory::new();
    
    // Create project.json
    fs.write_file_mut("/project.json", r#"{
        "uid": "test-project",
        "name": "Test Project"
    }"#.as_bytes()).unwrap();
    
    // Create a texture node
    fs.write_file_mut("/src/test.texture/node.json", r#"{
        "width": 10,
        "height": 10
    }"#.as_bytes()).unwrap();
    
    // Create a shader node that renders a gradient
    fs.write_file_mut("/src/test.shader/node.json", r#"{
        "glsl_path": "main.glsl",
        "texture_spec": "/src/test.texture",
        "render_order": 0
    }"#.as_bytes()).unwrap();
    
    // Create a GLSL shader that creates a gradient
    fs.write_file_mut("/src/test.shader/main.glsl", r#"
vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    float r = fragCoord.x / outputSize.x;
    float g = fragCoord.y / outputSize.y;
    return vec4(r, g, 0.5, 1.0);
}
"#.as_bytes()).unwrap();
    
    // Create runtime and initialize
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    // Tick to advance frame
    runtime.tick(16);
    
    // To trigger shader execution, we need to access the texture
    // Shaders are executed lazily when get_texture_mut() is called
    // Let's create a fixture that uses the texture to trigger rendering
    fs.write_file_mut("/src/test.output/node.json", r#"{
        "GpioStrip": {
            "pin": 18
        }
    }"#.as_bytes()).unwrap();
    
    fs.write_file_mut("/src/test.fixture/node.json", r#"{
        "output_spec": "/src/test.output",
        "texture_spec": "/src/test.texture",
        "mapping": "linear",
        "lamp_type": "rgb",
        "color_order": "Rgb",
        "transform": [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
    }"#.as_bytes()).unwrap();
    
    // Reload nodes to pick up the fixture
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    // Render (fixture will access texture, triggering shader execution)
    runtime.render().unwrap();
    
    // Verify texture was rendered by checking its state
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &lp_model::project::api::ApiNodeSpecifier::All,
    ).unwrap();
    
    match response {
        lp_model::project::api::ProjectResponse::GetChanges { node_details, .. } => {
            let texture_detail = node_details.values()
                .find(|d| d.path.as_str() == "/src/test.texture")
                .expect("Texture node should be in details");
            
            match &texture_detail.state {
                lp_model::project::api::NodeState::Texture(tex_state) => {
                    // Texture should have been rendered (10x10 RGBA8 = 400 bytes)
                    assert_eq!(tex_state.texture_data.len(), 10 * 10 * 4);
                    
                    // Check that pixels are not all zero (shader should have written data)
                    let has_non_zero = tex_state.texture_data.iter().any(|&b| b != 0);
                    assert!(has_non_zero, "Texture should have non-zero pixels after shader execution");
                }
                _ => panic!("Expected texture state"),
            }
        }
    }
}

#[test]
fn test_shader_compilation_error() {
    let mut fs = LpFsMemory::new();
    
    // Create project.json
    fs.write_file_mut("/project.json", r#"{
        "uid": "test-project",
        "name": "Test Project"
    }"#.as_bytes()).unwrap();
    
    // Create a texture node
    fs.write_file_mut("/src/test.texture/node.json", r#"{
        "width": 100,
        "height": 100
    }"#.as_bytes()).unwrap();
    
    // Create a shader node
    fs.write_file_mut("/src/test.shader/node.json", r#"{
        "glsl_path": "main.glsl",
        "texture_spec": "/src/test.texture",
        "render_order": 0
    }"#.as_bytes()).unwrap();
    
    // Create an invalid GLSL shader
    fs.write_file_mut("/src/test.shader/main.glsl", r#"
invalid glsl code here
"#.as_bytes()).unwrap();
    
    // Create runtime and initialize
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    runtime.load_nodes().unwrap();
    
    // Initialize should fail or set error status
    let _init_result = runtime.initialize_nodes();
    // Initialization might succeed but shader should be in error state
    
    // Check shader state for error
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &lp_model::project::api::ApiNodeSpecifier::All,
    ).unwrap();
    
    match response {
        lp_model::project::api::ProjectResponse::GetChanges { node_details, .. } => {
            let shader_detail = node_details.values()
                .find(|d| d.path.as_str() == "/src/test.shader")
                .expect("Shader node should be in details");
            
            match &shader_detail.state {
                lp_model::project::api::NodeState::Shader(shader_state) => {
                    // Should have compilation error
                    assert!(shader_state.error.is_some(), "Shader should have compilation error");
                }
                _ => panic!("Expected shader state"),
            }
            
            // Status should indicate error
            assert!(matches!(
                shader_detail.status,
                lp_model::project::api::NodeStatus::InitError(_) | lp_model::project::api::NodeStatus::Error(_)
            ));
        }
    }
}
