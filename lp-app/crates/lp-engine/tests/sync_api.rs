use lp_engine::ProjectRuntime;
use lp_engine::project::{ApiNodeSpecifier, ProjectResponse};
use lp_shared::fs::LpFsMemory;

#[test]
fn test_sync_api_get_changes() {
    // Create a minimal project
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
    
    // Create project runtime
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    
    // Load and initialize nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    // Get changes since frame 0 with All specifier
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &ApiNodeSpecifier::All,
    ).unwrap();
    
    match response {
        ProjectResponse::GetChanges {
            current_frame,
            node_handles,
            node_changes,
            node_details,
        } => {
            // Should have at least one node
            assert!(!node_handles.is_empty());
            
            // Should have details for all nodes (All specifier)
            assert_eq!(node_details.len(), node_handles.len());
            
            // Should have Created changes for new nodes (if loaded after frame 0)
            // Note: If nodes are loaded at frame 0, they may not show as Created
            // when querying since frame 0, which is expected behavior
            
            // Verify current_frame matches runtime frame_id
            assert_eq!(current_frame, runtime.frame_id);
        }
    }
}

#[test]
fn test_sync_api_no_changes() {
    // Create a minimal project
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
    
    // Create project runtime
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    
    // Load and initialize nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    let current_frame = runtime.frame_id;
    
    // Get changes since current frame (should have no changes)
    let response = runtime.get_changes(
        current_frame,
        &ApiNodeSpecifier::All,
    ).unwrap();
    
    match response {
        ProjectResponse::GetChanges {
            node_changes,
            ..
        } => {
            // Should have no changes since we're asking for changes since current frame
            assert!(node_changes.is_empty());
        }
    }
}

#[test]
fn test_sync_api_by_handles() {
    // Create a minimal project
    let mut fs = LpFsMemory::new();
    
    // Create project.json
    let project_json = r#"{
        "uid": "test-project",
        "name": "Test Project"
    }"#;
    fs.write_file_mut("/project.json", project_json.as_bytes()).unwrap();
    
    // Create two texture nodes
    let texture_json = r#"{
        "Memory": {
            "width": 100,
            "height": 100
        }
    }"#;
    fs.write_file_mut("/src/test1.texture/node.json", texture_json.as_bytes()).unwrap();
    fs.write_file_mut("/src/test2.texture/node.json", texture_json.as_bytes()).unwrap();
    
    // Create project runtime
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    
    // Load and initialize nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    
    // Get handles
    let handles: Vec<_> = runtime.nodes.keys().copied().collect();
    assert_eq!(handles.len(), 2);
    
    // Request details for only first handle
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &ApiNodeSpecifier::ByHandles(vec![handles[0]]),
    ).unwrap();
    
    match response {
        ProjectResponse::GetChanges {
            node_details,
            ..
        } => {
            // Should have detail for only the requested handle
            assert_eq!(node_details.len(), 1);
            assert!(node_details.contains_key(&handles[0]));
        }
    }
}
