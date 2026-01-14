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
    builder.shader(&texture_path).add(&mut builder);
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
    let frame1_data = client_view.get_texture_data(texture_handle).unwrap();
    assert_eq!(
        frame1_data.len(),
        16 * 16 * 4,
        "Texture should be 16x16 RGBA"
    );

    // Frame 1: After 16ms, time = 0.016s, mod(0.016, 1.0) = 0.016, red = 0.016 * 255 = 4.08 ≈ 4
    // Shader: vec4(mod(time, 1.0), 0.0, 0.0, 1.0) -> RGBA bytes [R, G, B, A]
    let expected_r1 = (0.016_f32 * 255.0) as u8; // = 4
    assert_pixel_rgba(&frame1_data, 0, expected_r1, 0, 0, 255, 1);

    // Frame 2: After 32ms, time = 0.032s, mod(0.032, 1.0) = 0.032, red = 0.032 * 255 = 8.16 ≈ 8
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    let frame2_data = client_view.get_texture_data(texture_handle).unwrap();
    let expected_r2 = (0.032_f32 * 255.0) as u8; // = 8
    assert_pixel_rgba(&frame2_data, 0, expected_r2, 0, 0, 255, 1);

    // Frame 3: After 48ms, time = 0.048s, mod(0.048, 1.0) = 0.048, red = 0.048 * 255 = 12.24 ≈ 12
    runtime.tick(16).unwrap();
    sync_client_view(&runtime, &mut client_view);
    let frame3_data = client_view.get_texture_data(texture_handle).unwrap();
    let expected_r3 = (0.048_f32 * 255.0) as u8; // = 12
    assert_pixel_rgba(&frame3_data, 0, expected_r3, 0, 0, 255, 1);

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

/// Assert pixel RGBA values with tolerance
/// 
/// Checks that the pixel at index `pixel_idx` in RGBA data has expected values.
/// Allows tolerance of ±1 for each channel to account for floating point precision.
fn assert_pixel_rgba(
    data: &[u8],
    pixel_idx: usize,
    expected_r: u8,
    expected_g: u8,
    expected_b: u8,
    expected_a: u8,
    tolerance: u8,
) {
    let offset = pixel_idx * 4;
    assert!(
        offset + 3 < data.len(),
        "Pixel index {} out of bounds for data length {}",
        pixel_idx,
        data.len()
    );

    let r = data[offset];
    let g = data[offset + 1];
    let b = data[offset + 2];
    let a = data[offset + 3];

    assert!(
        r.abs_diff(expected_r) <= tolerance,
        "Pixel {} R channel: expected {} ± {}, got {}",
        pixel_idx,
        expected_r,
        tolerance,
        r
    );
    assert!(
        g.abs_diff(expected_g) <= tolerance,
        "Pixel {} G channel: expected {} ± {}, got {}",
        pixel_idx,
        expected_g,
        tolerance,
        g
    );
    assert!(
        b.abs_diff(expected_b) <= tolerance,
        "Pixel {} B channel: expected {} ± {}, got {}",
        pixel_idx,
        expected_b,
        tolerance,
        b
    );
    assert!(
        a.abs_diff(expected_a) <= tolerance,
        "Pixel {} A channel: expected {} ± {}, got {}",
        pixel_idx,
        expected_a,
        tolerance,
        a
    );
}
