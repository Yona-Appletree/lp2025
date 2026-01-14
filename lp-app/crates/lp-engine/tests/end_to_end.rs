use lp_engine::ProjectRuntime;
use lp_engine_client::ClientProjectView;
use lp_model::project::ProjectBuilder;
use lp_shared::fs::LpFsMemory;

#[test]
fn test_end_to_end_shader_time_based() {
    let mut fs = LpFsMemory::new();
    let mut builder = ProjectBuilder::new(&mut fs);

    // Add nodes with defaults
    let texture_path = builder.texture_basic();
    builder.shader_basic(&texture_path);
    let output_path = builder.output_basic();
    builder.fixture_basic(&output_path, &texture_path);

    // Build project files
    builder.build();

    let mut runtime = ProjectRuntime::new(Box::new(fs)).unwrap();

    // Initialize all nodes
    runtime.load_nodes().unwrap();
    runtime.initialize_nodes().unwrap();
    runtime.ensure_all_nodes_initialized().unwrap();

    // Create client view
    let mut client_view = ClientProjectView::new();

    // Get output handle - we'll verify the full pipeline: shader -> texture -> fixture -> output
    let output_handle = runtime
        .resolve_path_to_handle(output_path.as_str())
        .unwrap();
    client_view.request_detail(vec![output_handle]);

    // Frame 1: After 16ms, time = 0.016s, mod(0.016, 1.0) = 0.016, red = 0.016 * 255 = 4.08 ≈ 4
    // Shader: vec4(mod(time, 1.0), 0.0, 0.0, 1.0) -> RGBA bytes [R, G, B, A]
    // Fixture samples texture and writes RGB to output channels
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    assert_first_output_rgb(
        &client_view.get_output_data(output_handle).unwrap(),
        (0.016_f32 * 255.0) as u8,
    );

    // Frame 2: After 32ms, time = 0.032s, mod(0.032, 1.0) = 0.032, red = 0.032 * 255 = 8.16 ≈ 8
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    assert_first_output_rgb(
        &client_view.get_output_data(output_handle).unwrap(),
        (0.032_f32 * 255.0) as u8,
    );

    // Frame 3: After 48ms, time = 0.048s, mod(0.048, 1.0) = 0.048, red = 0.048 * 255 = 12.24 ≈ 12
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    assert_first_output_rgb(
        &client_view.get_output_data(output_handle).unwrap(),
        (0.048_f32 * 255.0) as u8,
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

/// Assert first output channel RGB values
///
/// Output channels are RGB (3 bytes per channel). Checks that the first channel
/// has the expected RGB values (exact match).
fn assert_first_output_rgb(data: &[u8], expected_r: u8) {
    assert!(
        data.len() >= 3,
        "Output data should have at least 3 bytes (RGB) for first channel, got {}",
        data.len()
    );

    let r = data[0];
    let g = data[1];
    let b = data[2];

    // Shader outputs vec4(mod(time, 1.0), 0.0, 0.0, 1.0), so we expect red channel
    // to match time-based value exactly, green and blue to be 0
    assert_eq!(
        r, expected_r,
        "Output channel 0 R: expected {}, got {}",
        expected_r, r
    );
    assert_eq!(g, 0, "Output channel 0 G: expected 0, got {}", g);
    assert_eq!(b, 0, "Output channel 0 B: expected 0, got {}", b);
}
