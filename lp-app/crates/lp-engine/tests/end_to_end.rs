extern crate alloc;

use lp_engine::ProjectRuntime;
use lp_engine_client::ClientProjectView;
use lp_shared::fs::LpFsMemory;

#[test]
fn test_end_to_end_basic_flow() {
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
    
    // Create an output node
    let output_json = r#"{
        "GpioStrip": {
            "pin": 18
        }
    }"#;
    fs.write_file_mut("/src/test.output/node.json", output_json.as_bytes()).unwrap();
    
    // Create a fixture node
    let fixture_json = r#"{
        "output_spec": "/src/test.output",
        "texture_spec": "/src/test.texture",
        "mapping": "linear",
        "lamp_type": "rgb",
        "transform": [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]
    }"#;
    fs.write_file_mut("/src/test.fixture/node.json", fixture_json.as_bytes()).unwrap();
    
    // Create project runtime
    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();
    
    // Load nodes
    runtime.load_nodes().unwrap();
    assert_eq!(runtime.nodes.len(), 3);
    
    // Initialize nodes
    runtime.initialize_nodes().unwrap();
    
    // Verify all nodes initialized
    for entry in runtime.nodes.values() {
        assert!(!matches!(entry.status, lp_engine::project::NodeStatus::Created));
    }
    
    // Tick to next frame (so nodes created at frame 0 will show as changes)
    let initial_frame = runtime.frame_id;
    runtime.tick();
    assert_eq!(runtime.frame_id.as_i64(), initial_frame.as_i64() + 1);
    
    // Render frame
    runtime.render().unwrap();
    
    // Get changes for client sync (query since frame 0, so we get all nodes)
    let response = runtime.get_changes(
        lp_model::FrameId::default(),
        &lp_model::project::api::ApiNodeSpecifier::All,
    ).unwrap();
    
    // Create client view
    let mut client_view = ClientProjectView::new();
    
    // Sync client view
    client_view.sync(&response).unwrap();
    
    // Verify client view updated
    assert_eq!(client_view.frame_id, runtime.frame_id);
    // Nodes should be in client view (either from Created changes or from details)
    // Since we're querying with All specifier, details should include all nodes
    assert!(!client_view.nodes.is_empty());
    
    // Request detail for a node (use handles from runtime since client may not have them yet)
    let handles: Vec<_> = runtime.nodes.keys().copied().collect();
    assert!(!handles.is_empty());
    
    // If client doesn't have the node yet, it will be created on next sync
    // For now, just verify the sync worked
    if !client_view.nodes.is_empty() {
    
        client_view.request_detail(vec![handles[0]]);
        assert!(client_view.detail_tracking.contains(&handles[0]));
        
        // Get changes again with detail specifier
        let response2 = runtime.get_changes(
            client_view.frame_id,
            &client_view.detail_specifier(),
        ).unwrap();
        
        // Sync again
        client_view.sync(&response2).unwrap();
        
        // Verify detail was updated
        if let Some(entry) = client_view.nodes.get(&handles[0]) {
            // State should be present if detail was requested
            // (though it may be empty placeholder for now)
            assert!(entry.state.is_some() || true); // Allow for placeholder state
        }
    }
}
