use lp_engine::ProjectRuntime;
use lp_engine_client::ClientProjectView;
use lp_model::project::ProjectBuilder;
use lp_shared::fs::LpFsMemory;

#[test]
fn test_end_to_end_shader_time_based() {
    // Setup: Create project with shader that uses time (default sawtooth shader)
    let mut fs = LpFsMemory::new();
    let mut builder = ProjectBuilder::new(&mut fs).with_project("test-project", "Test Project");

    let texture_path = builder.texture().add(&mut builder); // Defaults to 16x16
    let shader_path = builder.shader(&texture_path).add(&mut builder);
    let output_path = builder.output().add(&mut builder); // Defaults to GPIO pin 0
    builder
        .fixture(&output_path, &texture_path)
        .add(&mut builder);
    builder.build();

    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();

    // Initialize all nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    runtime.ensure_all_nodes_initialized().unwrap();

    // Create client view
    let mut client_view = ClientProjectView::new();

    // Frame 1: Capture initial state
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);

    let texture_handle = runtime
        .resolve_path_to_handle(texture_path.as_str())
        .unwrap();
    client_view.request_detail(vec![texture_handle]);
    sync_client_view(&runtime, &mut client_view);

    // Get texture data from client view
    let frame1_data = {
        let entry = client_view.nodes.get(&texture_handle).unwrap();
        match &entry.state {
            Some(lp_model::project::api::NodeState::Texture(tex_state)) => {
                tex_state.texture_data.clone()
            }
            _ => panic!("Expected texture state"),
        }
    };
    assert_eq!(
        frame1_data.len(),
        16 * 16 * 4,
        "Texture should be 16x16 RGBA"
    );

    // Frame 2: Verify time-based change
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    let frame2_data = {
        let entry = client_view.nodes.get(&texture_handle).unwrap();
        match &entry.state {
            Some(lp_model::project::api::NodeState::Texture(tex_state)) => {
                tex_state.texture_data.clone()
            }
            _ => panic!("Expected texture state"),
        }
    };
    assert_ne!(
        &frame1_data[0..4],
        &frame2_data[0..4],
        "Texture should change with time (first pixel RGBA should differ)"
    );

    // Frame 3: Verify continued change
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    let frame3_data = {
        let entry = client_view.nodes.get(&texture_handle).unwrap();
        match &entry.state {
            Some(lp_model::project::api::NodeState::Texture(tex_state)) => {
                tex_state.texture_data.clone()
            }
            _ => panic!("Expected texture state"),
        }
    };
    assert_ne!(
        &frame1_data[0..4],
        &frame3_data[0..4],
        "Frame 3 should differ from Frame 1"
    );
    assert_ne!(
        &frame2_data[0..4],
        &frame3_data[0..4],
        "Frame 3 should differ from Frame 2"
    );

    // Verify client view frame_id matches runtime
    assert_eq!(client_view.frame_id, runtime.frame_id);
}

/// Sync the client view with the runtime
fn sync_client_view(runtime: &ProjectRuntime, client_view: &mut ClientProjectView) {
    let response = runtime
        .get_changes(
            client_view.frame_id,
            &lp_model::project::api::ApiNodeSpecifier::All,
        )
        .unwrap();
    client_view.sync(&response).unwrap();
}
