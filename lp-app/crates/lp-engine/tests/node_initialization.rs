use lp_engine::ProjectRuntime;
use lp_shared::fs::LpFsMemory;

#[test]
fn test_node_initialization() {
    // Create a minimal project in memory filesystem
    let mut fs = LpFsMemory::new();
    
    // Create project.json
    let project_json = r#"{
        "uid": "test-project",
        "name": "Test Project"
    }"#;
    fs.write_file_mut("/project.json", project_json.as_bytes()).unwrap();
    
    // Create a texture node
    let texture_json = r#"{
        "Memory": {
            "width": 100,
            "height": 100
        }
    }"#;
    fs.write_file_mut("/src/test.texture/node.json", texture_json.as_bytes()).unwrap();
    
    // Create a shader node
    let shader_json = r#"{
        "glsl_path": "main.glsl",
        "texture_spec": "/src/test.texture",
        "render_order": 0
    }"#;
    fs.write_file_mut("/src/test.shader/node.json", shader_json.as_bytes()).unwrap();
    
    // Create project runtime
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    
    // Load nodes
    runtime.load_nodes().unwrap();
    
    // Verify nodes were loaded
    assert_eq!(runtime.nodes.len(), 2);
    
    // Check that nodes were loaded (status should be Created or InitError if config failed)
    for entry in runtime.nodes.values() {
        // Nodes should be Created if loaded successfully, or InitError if config failed
        assert!(matches!(entry.status, 
            lp_engine::project::NodeStatus::Created | 
            lp_engine::project::NodeStatus::InitError(_)
        ));
        assert!(entry.runtime.is_none());
    }
    
    // Initialize nodes
    runtime.initialize_nodes().unwrap();
    
    // Verify nodes were initialized
    for entry in runtime.nodes.values() {
        // Should be Ok or InitError, not Created
        assert!(!matches!(entry.status, lp_engine::project::NodeStatus::Created));
        
        // If Ok, should have runtime
        if matches!(entry.status, lp_engine::project::NodeStatus::Ok) {
            assert!(entry.runtime.is_some());
        }
    }
}
