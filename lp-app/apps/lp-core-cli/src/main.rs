mod debug_ui;
mod fs;
mod led_output;
mod output_provider;
mod transport;
mod watcher;

use debug_ui::{render_fixtures_panel, render_shaders_panel, render_textures_panel};
use eframe::egui;
use fs::HostFilesystem;
use led_output::render_leds;
use lp_core::app::{LpApp, MsgIn, MsgOut, Platform};
use lp_core::error::Error;
use lp_core::traits::{LpFs, Transport};
use lp_core_util::fs::LpFsMemory;
use lp_server::ProjectManager;
use output_provider::HostOutputProvider;
use std::env;
use std::fs as std_fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use transport::HostTransport;
use watcher::FileWatcher;

/// Simple logger for host firmware that prints to stderr
///
/// Filters out logs from cranelift and other dependencies,
/// only showing logs from lp_core and fw-host.
struct HostLogger;

impl log::Log for HostLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Only show logs from our modules, filter out cranelift and other dependencies
        if let Some(target) = metadata.target().split("::").next() {
            matches!(target, "lp_core" | "fw_host")
        } else {
            true // If no target, show it (shouldn't happen)
        }
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let message = format!("{}", record.args());
            // For multi-line messages, preserve the original formatting
            // Errors like GlslError already have proper formatting with line numbers
            // and carets, so we just print each line as-is
            for line in message.lines() {
                eprintln!("[{}] {}", record.level(), line);
            }
        }
    }

    fn flush(&self) {
        // No-op for stderr
    }
}

/// Wrapper around HostOutputProvider to use it as a trait object
struct HostOutputProviderWrapper {
    inner: Arc<HostOutputProvider>,
}

impl lp_core::traits::OutputProvider for HostOutputProviderWrapper {
    fn create_output(
        &self,
        config: &lp_core::nodes::output::config::OutputNode,
        output_id: Option<lp_core::nodes::id::OutputId>,
    ) -> Result<std::boxed::Box<dyn lp_core::traits::LedOutput>, lp_core::error::Error> {
        self.inner.create_output(config, output_id)
    }
}

/// Collect messages from transport and convert to MsgIn
fn collect_messages(transport: &Arc<Mutex<dyn Transport>>) -> Vec<MsgIn> {
    let mut messages = Vec::new();

    // Try to receive messages (non-blocking, collect all available)
    // loop {
    //     let message = {
    //         let mut transport = transport.lock().unwrap();
    //         transport.receive_message()
    //     };
    //
    //     match message {
    //         Ok(msg) => match parse_command(&msg) {
    //             Ok(command) => {
    //                 let msg_in: MsgIn = command.into();
    //                 messages.push(msg_in);
    //             }
    //             Err(e) => {
    //                 log::warn!("Failed to parse command: {}", e);
    //                 break; // Stop on parse error
    //             }
    //         },
    //         Err(e) => {
    //             // "No message available" means no more messages
    //             if !e.to_string().contains("No message available") {
    //                 log::warn!("Error receiving message: {}", e);
    //             }
    //             break;
    //         }
    //     }
    // }

    messages
}

/// Handle outgoing messages from LpApp
fn handle_outgoing_messages(
    messages: Vec<MsgOut>,
    transport: &Arc<Mutex<dyn Transport>>,
) -> Result<(), Error> {
    for msg_out in messages {
        // match msg_out {
        //     MsgOut::Project { project } => {
        //         // Send project via transport
        //         let json = serde_json::to_string(&project)
        //             .map_err(|e| Error::Serialization(format!("{}", e)))?;
        //         let mut transport = transport.lock().unwrap();
        //         transport.send_message(&json)?;
        //     }
        // }
    }
    Ok(())
}

/// Parse command-line arguments
struct CliArgs {
    project_dir: Option<PathBuf>,
    create: bool,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = env::args().collect();
    let mut project_dir = None;
    let mut create = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--project-dir" | "-p" => {
                if i + 1 < args.len() {
                    project_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    eprintln!("Error: --project-dir requires a path argument");
                    std::process::exit(1);
                }
            }
            "--create" | "-c" => {
                create = true;
                i += 1;
            }
            "--help" | "-h" => {
                println!("LightPlayer Host Firmware");
                println!();
                println!("Usage: fw-host [<project-dir>] [OPTIONS]");
                println!();
                println!("Arguments:");
                println!(
                    "  <project-dir>              Project directory path (optional, defaults to in-memory testing mode)"
                );
                println!();
                println!("Options:");
                println!(
                    "  -p, --project-dir <path>  Specify project directory (alternative to positional argument)"
                );
                println!(
                    "  -c, --create              Create project if it doesn't exist (requires project-dir)"
                );
                println!("  -h, --help                Show this help message");
                println!();
                println!("Examples:");
                println!(
                    "  fw-host                                    # Run in testing mode (in-memory filesystem)"
                );
                println!(
                    "  fw-host ./my-project                     # Use project in ./my-project directory"
                );
                println!(
                    "  fw-host ./new-project --create           # Create new project in ./new-project"
                );
                std::process::exit(0);
            }
            arg => {
                // First positional argument is treated as project directory
                if project_dir.is_none() && !arg.starts_with("--") {
                    project_dir = Some(PathBuf::from(arg));
                    i += 1;
                } else {
                    eprintln!("Error: Unknown argument: {}", arg);
                    eprintln!("Use --help for usage information");
                    std::process::exit(1);
                }
            }
        }
    }

    CliArgs {
        project_dir,
        create,
    }
}

/// Create the test project files (texture, shader, output, fixture)
fn create_test_project_files(project_dir: &Path) -> Result<(), Error> {
    // Create src/ directory
    let src_dir = project_dir.join("src");
    std_fs::create_dir_all(&src_dir)
        .map_err(|e| Error::Filesystem(format!("Failed to create src directory: {}", e)))?;

    // Create texture node
    let texture_dir = src_dir.join("texture.texture");
    std_fs::create_dir_all(&texture_dir)
        .map_err(|e| Error::Filesystem(format!("Failed to create texture directory: {}", e)))?;
    std_fs::write(
        texture_dir.join("node.json"),
        br#"{"$type":"Memory","size":[64,64],"format":"RGB8"}"#,
    )
    .map_err(|e| Error::Filesystem(format!("Failed to write texture node.json: {}", e)))?;

    // Create shader node
    let shader_dir = src_dir.join("shader.shader");
    std_fs::create_dir_all(&shader_dir)
        .map_err(|e| Error::Filesystem(format!("Failed to create shader directory: {}", e)))?;
    std_fs::write(
        shader_dir.join("node.json"),
        br#"{"$type":"Single","texture_id":"/src/texture.texture"}"#,
    )
    .map_err(|e| Error::Filesystem(format!("Failed to write shader node.json: {}", e)))?;
    std_fs::write(
        shader_dir.join("main.glsl"),
        br#"// HSV to RGB conversion function
vec3 hsv_to_rgb(float h, float s, float v) {
    // h in [0, 1], s in [0, 1], v in [0, 1]
    float c = v * s;
    float x = c * (1.0 - abs(mod(h * 6.0, 2.0) - 1.0));
    float m = v - c;
    
    vec3 rgb;
    if (h < 1.0/6.0) {
        rgb = vec3(v, m + x, m);
    } else if (h < 2.0/6.0) {
        rgb = vec3(m + x, v, m);
    } else if (h < 3.0/6.0) {
        rgb = vec3(m, v, m + x);
    } else if (h < 4.0/6.0) {
        rgb = vec3(m, m + x, v);
    } else if (h < 5.0/6.0) {
        rgb = vec3(m + x, m, v);
    } else {
        rgb = vec3(v, m, m + x);
    }
    
    return rgb;
}

vec4 main(vec2 fragCoord, vec2 outputSize, float time) {
    // Center of texture
    vec2 center = outputSize * 0.5;
    
    // Direction from center to fragment
    vec2 dir = fragCoord - center;
    
    // Calculate angle (atan2 gives angle in [-PI, PI])
    float angle = atan(dir.y, dir.x);
    
    // Rotate angle with time (full rotation every 2 seconds)
    angle = (angle + time * 3.14159);
    
    // Normalize angle to [0, 1] for hue
    // atan returns [-PI, PI], map to [0, 1] by: (angle + PI) / (2 * PI)
    // Wrap hue to [0, 1] using mod to handle large time values
    float hue = mod((angle + 3.14159) / (2.0 * 3.14159), 1.0);
    
    // Distance from center (normalized to [0, 1])
    float maxDist = length(outputSize * 0.5);
    float dist = length(dir) / maxDist;
    
    // Clamp distance to prevent issues
    dist = min(dist, 1.0);
    
    // Value (brightness): highest at center, darker at edges
    float value = 1.0 - dist * 0.5;
    
    // Convert HSV to RGB
    vec3 rgb = hsv_to_rgb(hue, 1.0, value);
    
    // Clamp to [0, 1] and return
    return vec4(max(vec3(0.0), min(vec3(1.0), rgb)), 1.0);
}"#,
    )
    .map_err(|e| Error::Filesystem(format!("Failed to write shader main.glsl: {}", e)))?;

    // Create output node
    let output_dir = src_dir.join("output.output");
    std_fs::create_dir_all(&output_dir)
        .map_err(|e| Error::Filesystem(format!("Failed to create output directory: {}", e)))?;
    std_fs::write(
        output_dir.join("node.json"),
        br#"{"$type":"gpio_strip","chip":"ws2812","gpio_pin":4,"count":128}"#,
    )
    .map_err(|e| Error::Filesystem(format!("Failed to write output node.json: {}", e)))?;

    // Create fixture node
    let fixture_dir = src_dir.join("fixture.fixture");
    std_fs::create_dir_all(&fixture_dir)
        .map_err(|e| Error::Filesystem(format!("Failed to create fixture directory: {}", e)))?;
    std_fs::write(
        fixture_dir.join("node.json"),
        br#"{"$type":"circle-list","output_id":"/src/output.output","texture_id":"/src/texture.texture","channel_order":"rgb","mapping":[{"channel":0,"center":[0.03125,0.0625],"radius":0.05},{"channel":1,"center":[0.09375,0.0625],"radius":0.05},{"channel":2,"center":[0.15625,0.0625],"radius":0.05},{"channel":3,"center":[0.21875,0.0625],"radius":0.05},{"channel":4,"center":[0.28125,0.0625],"radius":0.05},{"channel":5,"center":[0.34375,0.0625],"radius":0.05},{"channel":6,"center":[0.40625,0.0625],"radius":0.05},{"channel":7,"center":[0.46875,0.0625],"radius":0.05},{"channel":8,"center":[0.53125,0.0625],"radius":0.05},{"channel":9,"center":[0.59375,0.0625],"radius":0.05},{"channel":10,"center":[0.65625,0.0625],"radius":0.05},{"channel":11,"center":[0.71875,0.0625],"radius":0.05}]}"#,
    )
    .map_err(|e| Error::Filesystem(format!("Failed to write fixture node.json: {}", e)))?;

    Ok(())
}

/// Create a new project structure in the specified directory
fn create_project(project_dir: &Path) -> Result<(), Error> {
    let project_json_path = project_dir.join("project.json");

    // Check if project already exists
    if project_json_path.exists() {
        return Err(Error::Filesystem(format!(
            "Project already exists at {:?}",
            project_dir
        )));
    }

    // Create project directory if it doesn't exist
    std_fs::create_dir_all(project_dir).map_err(|e| {
        Error::Filesystem(format!(
            "Failed to create project directory {:?}: {}",
            project_dir, e
        ))
    })?;

    // Create project.json with default config
    let default_config = lp_core::project::config::ProjectConfig {
        uid: "default".to_string(),
        name: "Default Project".to_string(),
    };
    let json = serde_json::to_string_pretty(&default_config)
        .map_err(|e| Error::Serialization(format!("Failed to serialize project config: {}", e)))?;
    std_fs::write(&project_json_path, json.as_bytes())
        .map_err(|e| Error::Filesystem(format!("Failed to write project.json: {}", e)))?;

    // Create test project files (texture, shader, output, fixture)
    create_test_project_files(project_dir)?;

    log::info!("Created new project in {:?}", project_dir);
    log::info!("  - project.json");
    log::info!("  - src/");
    log::info!("  - src/texture.texture/");
    log::info!("  - src/shader.shader/");
    log::info!("  - src/output.output/");
    log::info!("  - src/fixture.fixture/");

    Ok(())
}

fn main() -> eframe::Result<()> {
    // Initialize logger early
    let logger = Box::new(HostLogger);
    log::set_logger(Box::leak(logger))
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to set logger");

    // Parse command-line arguments
    let args = parse_args();

    // Determine project setup
    let (projects_base_dir, project_name, use_watcher, project_root) =
        if let Some(project_dir) = args.project_dir {
            // Use real filesystem with specified project directory
            let project_root = project_dir.canonicalize().unwrap_or(project_dir.clone());

            // Extract project name from directory name
            let project_name = project_root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("current")
                .to_string();

            // Use parent directory as projects base, or current directory if no parent
            let projects_base_dir = project_root
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());

            // Handle --create flag
            if args.create {
                // Create filesystem at server root
                let server_root = PathBuf::from(&projects_base_dir);
                let fs: Box<dyn LpFs> = Box::new(HostFilesystem::new(server_root.clone()));

                // Initialize output provider
                let output_provider = Arc::new(HostOutputProvider::new());
                let platform_output_provider: Box<dyn lp_core::traits::OutputProvider> =
                    Box::new(HostOutputProviderWrapper {
                        inner: Arc::clone(&output_provider),
                    });
                let platform = Platform::new(fs, platform_output_provider);

                // Create ProjectManager and create project
                let mut project_manager = ProjectManager::new(projects_base_dir.clone());
                match project_manager.create_project(project_name.clone(), platform) {
                    Ok(()) => {
                        log::info!("Project created successfully!");
                    }
                    Err(e) => {
                        eprintln!("Error creating project: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                // Check if project.json exists
                let project_json_path = project_root.join("project.json");
                if !project_json_path.exists() {
                    eprintln!("Error: Project not found at {:?}", project_root);
                    eprintln!("Use --create to create a new project");
                    std::process::exit(1);
                }
            }

            (projects_base_dir, project_name, true, project_root)
        } else {
            // Use in-memory filesystem with sample project (testing mode)
            log::info!("Running in testing mode (in-memory filesystem with sample project)");
            log::info!("Use --project-dir <path> to use a real project directory");

            // For in-memory, use "." as base and "test" as project name
            (
                "./test-projects".to_string(),
                "test".to_string(),
                false,
                PathBuf::from("."),
            )
        };

    // Initialize output provider
    let output_provider = Arc::new(HostOutputProvider::new());
    let platform_output_provider: Box<dyn lp_core::traits::OutputProvider> =
        Box::new(HostOutputProviderWrapper {
            inner: Arc::clone(&output_provider),
        });

    // Create filesystem at server root
    let filesystem: Box<dyn LpFs> = if use_watcher {
        Box::new(HostFilesystem::new(PathBuf::from(&projects_base_dir)))
    } else {
        // For in-memory mode, create filesystem with sample project
        // ProjectManager expects: projects_base_dir = "./test-projects", project_name = "test"
        // So files should be at: /test-projects/test/project.json
        let mut fs = LpFsMemory::new();
        fs.write_file_mut(
            "/test-projects/test/project.json",
            br#"{"uid":"test","name":"Test Project"}"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/test-projects/test/src/texture.texture/node.json",
            br#"{"$type":"Memory","size":[64,64],"format":"RGB8"}"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/test-projects/test/src/shader.shader/node.json",
            br#"{"$type":"Single","texture_id":"/src/texture.texture"}"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/test-projects/test/src/shader.shader/main.glsl",
            br#"vec4 main(vec2 fragCoord, vec2 outputSize, float time) { return vec4(1.0); }"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/test-projects/test/src/output.output/node.json",
            br#"{"$type":"gpio_strip","chip":"ws2812","gpio_pin":4,"count":128}"#,
        )
        .unwrap();
        fs.write_file_mut(
            "/test-projects/test/src/fixture.fixture/node.json",
            br#"{"$type":"circle-list","output_id":"/src/output.output","texture_id":"/src/texture.texture","channel_order":"rgb","mapping":[]}"#,
        )
        .unwrap();
        Box::new(fs)
    };

    let platform = Platform::new(filesystem, platform_output_provider);

    // Create ProjectManager and load project
    let mut project_manager = ProjectManager::new(projects_base_dir.clone());

    // Wrap in catch_unwind to handle panics from cranelift
    log::info!(
        "Loading project '{}' from base directory '{}'",
        project_name,
        projects_base_dir
    );
    let load_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        project_manager.load_project(project_name.clone(), platform)
    }));

    match load_result {
        Ok(Ok(())) => {
            log::info!("Project loaded successfully");
        }
        Ok(Err(e)) => {
            log::error!("Failed to load project '{}': {}", project_name, e);
            eprintln!("Error: Failed to load project: {}", e);
            std::process::exit(1);
        }
        Err(panic_info) => {
            log::warn!(
                "Project loading panicked (possibly due to unimplemented platform features in cranelift)"
            );
            log::warn!("This is a known issue on macOS - shader compilation may not work");
            eprintln!("Warning: Project loading panicked");
            // Continue anyway - the app will show errors in the debug UI
        }
    }

    // Initialize filesystem watcher (only for real filesystem)
    let file_watcher = if use_watcher {
        match FileWatcher::watch_project(project_root) {
            Ok(watcher) => {
                log::info!("Filesystem watcher initialized");
                Some(watcher)
            }
            Err(e) => {
                log::warn!("Failed to initialize filesystem watcher: {}", e);
                log::warn!("File changes will not be detected automatically");
                None
            }
        }
    } else {
        // No watcher for in-memory filesystem
        None
    };

    // Initialize transport (stdio)
    let transport: Arc<Mutex<dyn Transport>> = Arc::new(Mutex::new(HostTransport::new()));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("LightPlayer Host Firmware"),
        ..Default::default()
    };

    eframe::run_native(
        "LightPlayer Host",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(AppState {
                project_manager,
                current_project_name: project_name,
                transport,
                output_provider,
                file_watcher,
                selected_led: None,
                last_frame_time: None,
                frame_count: 0,
                fps_history: Vec::new(),
            }))
        }),
    )
}

struct AppState {
    project_manager: ProjectManager,
    current_project_name: String,
    transport: Arc<Mutex<dyn Transport>>,
    output_provider: Arc<HostOutputProvider>,
    file_watcher: Option<FileWatcher>,
    selected_led: Option<usize>,
    last_frame_time: Option<Instant>,
    frame_count: u64,
    fps_history: Vec<f32>, // Store last N frame times for average FPS
}

impl AppState {
    /// Get mutable access to the current project's LpApp
    fn lp_app_mut(&mut self) -> Option<&mut LpApp> {
        self.project_manager
            .get_project_mut(&self.current_project_name)
            .map(|p| p.app_mut())
    }

    /// Get immutable access to the current project's LpApp
    fn lp_app(&self) -> Option<&LpApp> {
        self.project_manager
            .get_project(&self.current_project_name)
            .map(|p| p.app())
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Calculate delta_ms from frame timestamps
        let now = Instant::now();
        let delta_ms = if let Some(last_time) = self.last_frame_time {
            let delta = now.duration_since(last_time);
            delta.as_millis().min(u32::MAX as u128) as u32
        } else {
            0 // First frame, no delta
        };
        self.last_frame_time = Some(now);

        // Update frame count
        self.frame_count += 1;

        // Calculate FPS (frames per second) and update history
        let current_fps = if delta_ms > 0 {
            1000.0 / delta_ms as f32
        } else {
            0.0
        };

        // Update FPS history (keep last 60 frames for average)
        self.fps_history.push(current_fps);
        if self.fps_history.len() > 60 {
            self.fps_history.remove(0);
        }

        // Collect messages from transport
        let incoming_messages = collect_messages(&self.transport);

        // Get file changes from watcher
        let file_changes: Vec<_> = self
            .file_watcher
            .as_ref()
            .map(|watcher| watcher.get_changes())
            .unwrap_or_default();

        // Log file changes if any
        if !file_changes.is_empty() {
            for change in &file_changes {
                log::debug!("File change: {:?} - {}", change.change_type, change.path);
            }
        }

        // Update runtime with tick() - processes messages and updates runtime
        let tick_result = if let Some(lp_app) = self.lp_app_mut() {
            lp_app.tick(delta_ms, &incoming_messages, &file_changes)
        } else {
            log::error!("No project loaded");
            return;
        };

        match tick_result {
            Ok(outgoing) => {
                // Handle outgoing messages
                if let Err(e) = handle_outgoing_messages(outgoing, &self.transport) {
                    log::error!("Error handling outgoing messages: {}", e);
                }
            }
            Err(e) => {
                log::error!("Error in tick: {}", e);
            }
        }

        // Request repaint to keep loop running continuously
        // Use request_repaint_after to ensure we get regular updates even if nothing changes
        ctx.request_repaint_after(std::time::Duration::from_millis(16)); // ~60 FPS

        // Use a side panel for textures and main panel for LEDs
        egui::SidePanel::right("debug_panel")
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Debug Panel");
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        if let Some(lp_app) = self.lp_app() {
                            if let Some(project) = lp_app.config() {
                                // Show project info
                                ui.group(|ui| {
                                    ui.label(format!("Project: {}", project.name));
                                    ui.label(format!("UID: {}", project.uid));
                                });
                                ui.separator();

                                // Show textures
                                render_textures_panel(ui, project, lp_app.runtime());

                                ui.separator();

                                // Show shaders
                                render_shaders_panel(ui, project, lp_app.runtime());

                                ui.separator();

                                // Show fixtures
                                render_fixtures_panel(ui, project, lp_app.runtime());
                            } else {
                                ui.label("No project config loaded");
                            }
                        } else {
                            ui.label("No project loaded");
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("LightPlayer Host Firmware");
            ui.separator();

            // Frame statistics
            ui.group(|ui| {
                ui.heading("Frame Statistics");

                let current_fps = self.fps_history.last().copied().unwrap_or(0.0);
                let avg_fps = if !self.fps_history.is_empty() {
                    self.fps_history.iter().sum::<f32>() / self.fps_history.len() as f32
                } else {
                    0.0
                };

                ui.label(format!("Frame: {}", self.frame_count));
                ui.label(format!("Current FPS: {:.1}", current_fps));
                ui.label(format!("Average FPS: {:.1}", avg_fps));

                if let Some(lp_app) = self.lp_app() {
                    if let Some(runtime) = lp_app.runtime() {
                        let frame_time = runtime.frame_time();
                        ui.label(format!(
                            "Total Time: {:.2}s",
                            frame_time.total_ms as f32 / 1000.0
                        ));
                        ui.label(format!("Delta: {}ms", frame_time.delta_ms));
                    } else {
                        ui.label("Runtime not initialized");
                    }
                } else {
                    ui.label("No project loaded");
                }
            });
            ui.separator();

            // Show project info
            if let Some(lp_app) = self.lp_app() {
                if let Some(project) = lp_app.config() {
                    ui.label(format!("Project: {} ({})", project.name, project.uid));
                } else {
                    ui.label("No project config loaded");
                }
            } else {
                ui.label("No project loaded");
            }

            ui.separator();

            // Show outputs - one section per output
            let outputs = self.output_provider.get_all_outputs();
            if outputs.is_empty() {
                ui.label("No outputs configured");
            } else {
                for (output_id, output_arc) in &outputs {
                    {
                        let output = output_arc.lock().unwrap();
                        let pixel_count = lp_core::traits::LedOutput::get_pixel_count(&*output);
                        ui.heading(format!(
                            "Output {} ({} LEDs)",
                            String::from(output_id.clone()),
                            pixel_count
                        ));
                    }
                    ui.separator();

                    // Render LEDs for this output in a contained panel
                    let output = output_arc.lock().unwrap();
                    let pixel_count = lp_core::traits::LedOutput::get_pixel_count(&*output);

                    // Calculate size needed for LED grid
                    let cols = (pixel_count as f32).sqrt().ceil() as usize;
                    let led_size = 25.0;
                    let spacing = 8.0;
                    let estimated_width = (cols as f32) * (led_size + spacing) - spacing;
                    let estimated_height =
                        ((pixel_count + cols - 1) / cols) as f32 * (led_size + spacing) - spacing;

                    // Create contained area with clipping
                    let available_width = ui.available_width();
                    let max_height = 400.0;
                    let allocated_size = egui::Vec2::new(
                        available_width.min(estimated_width + 20.0),
                        estimated_height.min(max_height) + 20.0,
                    );

                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.set_max_size(allocated_size);
                            ui.set_clip_rect(ui.max_rect());
                            render_leds(ui, &*output, self.selected_led);
                        });
                    drop(output);

                    ui.separator();
                }
            }
        });
    }
}
