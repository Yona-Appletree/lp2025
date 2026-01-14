use lp_engine::ProjectRuntime;
use lp_engine_client::ClientProjectView;
use lp_model::project::ProjectBuilder;
use lp_shared::fs::LpFsMemory;

/// Setup a project with a shader that uses time
fn setup_project_with_shader() -> ProjectRuntime {
    let mut fs = LpFsMemory::new();
    let mut builder = ProjectBuilder::new(&mut fs)
        .with_project("test-project", "Test Project");

    let texture_path = builder.texture(100, 100).add(&mut builder);
    
    builder.shader(&texture_path)
        .glsl("vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    float r = (sin(time) + 1.0) * 0.5;
    float g = (cos(time * 1.5) + 1.0) * 0.5;
    float b = (sin(time * 2.0) + 1.0) * 0.5;
    return vec4(r, g, b, 1.0);
}")
        .add(&mut builder);
    
    let output_path = builder.output()
        .gpio_pin(18)
        .add(&mut builder);
    
    builder.fixture(&output_path, &texture_path).add(&mut builder);

    builder.build();
    ProjectRuntime::new(Box::new(fs)).unwrap()
}

/// Tick the runtime and render a frame
fn tick_and_render(runtime: &mut ProjectRuntime, delta_ms: u32) {
    runtime.tick(delta_ms);
    runtime.render().unwrap();
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

/// Find a node handle by path
fn find_handle_by_path(runtime: &ProjectRuntime, path: &str) -> lp_model::NodeHandle {
    let path = lp_model::LpPath::from(path);
    runtime
        .nodes
        .iter()
        .find(|(_, entry)| entry.path == path)
        .map(|(handle, _)| *handle)
        .expect("Node not found")
}

/// Get texture data from client view
fn get_texture_data(client_view: &ClientProjectView, texture_path: &str) -> Vec<u8> {
    let handle = client_view
        .nodes
        .iter()
        .find(|(_, entry)| entry.path.as_str() == texture_path)
        .map(|(handle, _)| *handle)
        .expect("Texture node not found in client view");

    let entry = client_view
        .nodes
        .get(&handle)
        .expect("Node entry not found");

    match &entry.state {
        Some(lp_model::project::api::NodeState::Texture(tex_state)) => {
            tex_state.texture_data.clone()
        }
        _ => panic!("Expected texture state"),
    }
}

#[test]
fn test_end_to_end_shader_time_based() {
    // Setup: Create project with shader that uses time
    let mut runtime = setup_project_with_shader();

    // Initialize all nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();

    // Verify shader initialized successfully
    let shader_handle = find_handle_by_path(&runtime, "/src/shader-1.shader");
    let shader_entry = runtime.nodes.get(&shader_handle).unwrap();
    assert!(
        matches!(shader_entry.status, lp_engine::project::NodeStatus::Ok),
        "Shader should initialize successfully"
    );

    // Create client view
    let mut client_view = ClientProjectView::new();

    // Frame 1: Capture initial state
    tick_and_render(&mut runtime, 16);
    sync_client_view(&runtime, &mut client_view);

    let texture_path = "/src/texture-1.texture";
    let texture_handle = find_handle_by_path(&runtime, texture_path);
    client_view.request_detail(vec![texture_handle]);
    sync_client_view(&runtime, &mut client_view);

    let frame1_data = get_texture_data(&client_view, texture_path);
    assert_eq!(frame1_data.len(), 100 * 100 * 4, "Texture should be 100x100 RGBA");

    // Frame 2: Verify time-based change
    tick_and_render(&mut runtime, 16);
    sync_client_view(&runtime, &mut client_view);
    let frame2_data = get_texture_data(&client_view, texture_path);
    assert_ne!(
        &frame1_data[0..4],
        &frame2_data[0..4],
        "Texture should change with time (first pixel RGBA should differ)"
    );

    // Frame 3: Verify continued change
    tick_and_render(&mut runtime, 16);
    sync_client_view(&runtime, &mut client_view);
    let frame3_data = get_texture_data(&client_view, texture_path);
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
