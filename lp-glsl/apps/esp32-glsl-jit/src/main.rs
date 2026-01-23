//! embassy hello world with hashbrown test
//!
//! This is an example of running the embassy executor with hashbrown to test
//! alloc conflict with build-std.

#![no_std]
#![no_main]

extern crate alloc;

mod jit_fns;
mod shader_call;

use alloc::string::String;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_backtrace as _;
use esp_hal::{interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup};
use hashbrown::HashMap;

use cranelift_codegen::isa::riscv32::isa_builder;
use cranelift_codegen::settings::{self, Configurable};
use lp_builtins::fixed32::q32::Q32;
use lp_glsl_compiler::Compiler;
use lp_glsl_compiler::backend::transform::fixed32::{Fixed32Transform, FixedPointFormat};
use target_lexicon::Triple;

use esp_println::println;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run() {
    loop {
        esp_println::println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Allocate heap
    esp_alloc::heap_allocator!(size: 256 * 1024);

    println!("Init!");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // Test hashbrown HashMap
    let mut map: HashMap<String, u32> = HashMap::new();
    map.insert(String::from("test"), 42);
    println!("HashMap test: {:?}", map.get("test"));

    spawner.spawn(run()).ok();

    // loop {
    //     println!("Bing!");
    //     Timer::after(Duration::from_millis(5_000)).await;
    // }

    println!("======================================");
    println!("ESP32-C6 GLSL JIT Test");
    println!("Testing Cranelift GLSL Compiler on Real RISC-V Hardware!");
    println!("======================================\n");

    // Fragment shader: pattern generator that takes pixel coordinates
    // This simulates real image rendering where each pixel is computed independently
    // Note: Using main() with parameters (non-standard GLSL, but supported by our compiler)
    let source = r#"
   // Hash function for pseudo-random gradient vectors
// Adapted for fixed-point arithmetic (clamped at 2^16)
float hash(vec2 p) {
    // Use smaller constants to avoid exceeding 2^16 limit
    // First compute dot product and keep it in reasonable range
    float h = dot(p, vec2(12.9898, 78.233));
    // Use mod to wrap large values, keeping within safe range
    h = mod(h, 1000.0);
    // Use smaller multiplier to ensure result stays well under 2^16
    // sin returns [-1, 1], so max result is 10000.0, well under 65536
    return fract(sin(h) * 10000.0);
}

// Smooth interpolation function
float smoothf(float t) {
    return t * t * (3.0 - 2.0 * t);
}

// 2D Perlin noise function
float perlin_noise(vec2 p) {
    // Get integer coordinates of the grid cell
    vec2 i = floor(p);
    vec2 f = fract(p);

    // Get hash values for the four corners
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));

    // Create gradient vectors from hash values
    vec2 grad_a = vec2(cos(a * 6.28318), sin(a * 6.28318));
    vec2 grad_b = vec2(cos(b * 6.28318), sin(b * 6.28318));
    vec2 grad_c = vec2(cos(c * 6.28318), sin(c * 6.28318));
    vec2 grad_d = vec2(cos(d * 6.28318), sin(d * 6.28318));

    // Distance vectors from corners to point
    vec2 dist_a = f;
    vec2 dist_b = f - vec2(1.0, 0.0);
    vec2 dist_c = f - vec2(0.0, 1.0);
    vec2 dist_d = f - vec2(1.0, 1.0);

    // Dot products (gradient * distance)
    float dot_a = dot(grad_a, dist_a);
    float dot_b = dot(grad_b, dist_b);
    float dot_c = dot(grad_c, dist_c);
    float dot_d = dot(grad_d, dist_d);

    // Smooth interpolation
    float u = smoothf(f.x);
    float v = smoothf(f.y);

    // Bilinear interpolation
    float x1 = mix(dot_a, dot_b, u);
    float x2 = mix(dot_c, dot_d, u);
    float result = mix(x1, x2, v);

    // Normalize to approximately [0, 1] range
    return result * 0.5 + 0.5;
}

// HSV to RGB conversion function
vec3 hsv_to_rgb(float h, float s, float v) {
    // h in [0, 1], s in [0, 1], v in [0, 1]
    float c = v * s;
    float x = c * (1.0 - abs(mod(h * 6.0, 2.0) - 1.0));
    float m = v - c;

    vec3 rgb;
    if (h < 1.0 / 6.0) {
        rgb = vec3(v, m + x, m);
    } else if (h < 2.0 / 6.0) {
        rgb = vec3(m + x, v, m);
    } else if (h < 3.0 / 6.0) {
        rgb = vec3(m, v, m + x);
    } else if (h < 4.0 / 6.0) {
        rgb = vec3(m, m + x, v);
    } else if (h < 5.0 / 6.0) {
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

    // Normalize coordinates to [0, 1] range for noise sampling
    vec2 uv = fragCoord / outputSize;

    // Zoom through noise using time with oscillation to stay bounded
    // Oscillate between minZoom and maxZoom to avoid unbounded growth
    float minZoom = 1.0;
    float maxZoom = 8.0;
    float zoomSpeed = 0.5;
    // Use sine to oscillate between min and max zoom
    // sin returns [-1, 1], map to [minZoom, maxZoom]
    float zoom = minZoom + (maxZoom - minZoom) * 0.5 * (sin(time * zoomSpeed) + 1.0);

    // Sample Perlin noise with zoom
    vec2 noiseCoord = uv * zoom;
    float noise = perlin_noise(noiseCoord);

    // Apply cosine to the noise and normalize to [0, 1] for hue
    float cosNoise = cos(noise * 6.28318); // Multiply by 2*PI for full cycle
    float hue = (cosNoise + 1.0) * 0.5; // Map from [-1, 1] to [0, 1]

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
}
    "#;

    println!("======================================");
    println!("GLSL Shader Program:");
    println!("======================================");

    // Build the formatted program as a single string to output in chunks
    println!("{}", source);

    println!("======================================");
    println!("End of GLSL program ({} lines)", source.lines().count());
    println!("");

    // Create RISC-V32 ISA
    println!("Step 1: Creating RISC-V32 ISA...");
    let mut flag_builder = settings::builder();
    flag_builder.set("opt_level", "none").unwrap();
    flag_builder.set("is_pic", "false").unwrap();
    flag_builder.set("enable_verifier", "false").unwrap();
    let isa_flags = settings::Flags::new(flag_builder);

    let triple = Triple {
        architecture: target_lexicon::Architecture::Riscv32(
            target_lexicon::Riscv32Architecture::Riscv32imac,
        ),
        vendor: target_lexicon::Vendor::Unknown,
        operating_system: target_lexicon::OperatingSystem::None_,
        environment: target_lexicon::Environment::Unknown,
        binary_format: target_lexicon::BinaryFormat::Elf,
    };

    let isa = match isa_builder(triple).finish(isa_flags) {
        Ok(isa) => {
            println!("  ✓ ISA created");
            isa
        }
        Err(_e) => {
            panic!("ISA creation failed");
        }
    };

    // Compile GLSL using normal JIT path
    println!("Step 2: Compiling GLSL to RISC-V machine code...");
    let mut compiler = Compiler::new();

    // Create a Target from the ISA for JIT compilation
    use lp_glsl_compiler::backend::target::Target;
    let flags = isa.flags().clone();
    let target = Target::HostJit {
        arch: None, // Auto-detect from host (ESP32 = RISC-V32)
        flags,
        isa: None, // Will be created from target
    };

    // Compile to JIT module
    let gl_module = match compiler.compile_to_gl_module_jit(source, target) {
        Ok(module) => {
            println!("  ✓ GLSL compilation successful");
            module
        }
        Err(e) => {
            // Build error message string
            use alloc::format;
            let mut error_msg =
                format!("GLSL compilation failed!\nError: {}\n", e.message.as_str());

            if let Some(ref loc) = e.location {
                error_msg.push_str(&format!("At line {}, column {}\n", loc.line, loc.column));
            }

            if let Some(ref span) = e.span_text {
                error_msg.push_str(&format!("Source code: {}\n", span.as_str()));
            }

            if !e.notes.is_empty() {
                error_msg.push_str("Error details:\n");
                for note in &e.notes {
                    for line in note.lines() {
                        if !line.trim().is_empty() {
                            error_msg.push_str(&format!("  {}\n", line));
                        }
                    }
                }
            }

            panic!("{}", error_msg.as_str());
        }
    };

    // Apply fixed32 transform to convert f32 operations to fixed-point (i32)
    println!("Step 2b: Applying fixed32 transform...");
    let gl_module =
        match gl_module.apply_transform(Fixed32Transform::new(FixedPointFormat::Fixed16x16)) {
            Ok(module) => {
                println!("  ✓ Fixed32 transform applied");
                module
            }
            Err(e) => {
                println!("Failed to apply fixed32 transform: {}", e.message.as_str());
                panic!("Fixed32 transform failed");
            }
        };

    // Build JIT executable directly to get the concrete type with function pointers
    use lp_glsl_compiler::backend::codegen::jit::build_jit_executable;
    let jit_module = match build_jit_executable(gl_module) {
        Ok(module) => {
            println!("  ✓ JIT executable built");
            module
        }
        Err(e) => {
            println!("Failed to build executable: {}", e.message.as_str());
            panic!("JIT executable build failed");
        }
    };

    // Get the function pointer for "main"
    let func_ptr = match jit_module.get_function_ptr("main") {
        Ok(ptr) => {
            println!("  ✓ Function pointer obtained");
            ptr
        }
        Err(e) => {
            println!("Failed to get function pointer: {}", e.message.as_str());
            panic!("Function pointer not found");
        }
    };

    println!("Step 3: Setting up continuous rendering loop...");

    // Ensure instruction cache coherency
    unsafe {
        core::arch::asm!("fence.i");
    }

    // Image dimensions: 64x64 pixels = 4096 pixels per frame
    const IMAGE_WIDTH: i32 = 64;
    const IMAGE_HEIGHT: i32 = 64;
    const PIXELS_PER_FRAME: u32 = (IMAGE_WIDTH * IMAGE_HEIGHT) as u32;

    // Convert to fixed32 using Q32
    let output_size = [
        Q32::from_i32(IMAGE_WIDTH).to_fixed(),  // width in fixed32
        Q32::from_i32(IMAGE_HEIGHT).to_fixed(), // height in fixed32
    ];

    println!(
        "Rendering {}x{} image ({} pixels per frame)",
        IMAGE_WIDTH, IMAGE_HEIGHT, PIXELS_PER_FRAME
    );
    println!("Starting continuous rendering loop...\n");

    let mut frame_count: u32 = 0;
    let mut last_fps_report = Instant::now();
    const FPS_REPORT_INTERVAL_MS: u64 = 2000; // Report FPS every 2 seconds
    let mut time = Q32::ZERO; // Time in fixed32
    // TIME_STEP = 0.016 seconds (~60 FPS) in fixed32
    // Using Q32::from_f32 would require f32, so we construct directly
    // 0.016 * 65536 = 1048.576, rounded to 1049
    const TIME_STEP: Q32 = Q32(1049);

    // Continuous rendering loop
    loop {
        // Render one frame (all pixels)
        let frame_start = Instant::now();

        // Render all pixels in the frame
        for y in 0..IMAGE_HEIGHT {
            for x in 0..IMAGE_WIDTH {
                // Call shader main function directly
                // Signature: vec4 main(vec2 fragCoord, vec2 outputSize, float time)
                // All values are in fixed32 format (i32)
                let frag_coord = [Q32::from_i32(x).to_fixed(), Q32::from_i32(y).to_fixed()];
                let [r, g, b, a] = unsafe {
                    shader_call::call_vec4_shader(
                        func_ptr,
                        frag_coord,
                        output_size,
                        time.to_fixed(),
                        &isa,
                    )
                    .unwrap_or_else(|e| {
                        panic!("Shader call failed: {:?}", e);
                    })
                };

                // r, g, b, a are fixed32 (i32) values
                // In a real implementation, we would store these in a framebuffer
                // For now, we just compute it to test the shader
                let _ = (r, g, b, a);
            }
        }

        // Update time for next frame
        time = time + TIME_STEP;

        let frame_end = Instant::now();
        let frame_time = frame_end.duration_since(frame_start);
        frame_count += 1;

        // Report FPS periodically
        let time_since_last_report = frame_end.duration_since(last_fps_report);
        if time_since_last_report.as_millis() >= FPS_REPORT_INTERVAL_MS {
            // Calculate FPS: frames per second = frame_count / elapsed_seconds
            let elapsed_ms = time_since_last_report.as_millis();
            let elapsed_seconds = elapsed_ms as f32 / 1000.0;
            let fps = frame_count as f32 / elapsed_seconds;

            // Format FPS with 2 decimal places
            let fps_int = (fps * 100.0) as u32;
            let fps_whole = fps_int / 100;
            let fps_frac = fps_int % 100;

            println!(
                "FPS: {}.{:02} | Frame time: {} ms | Pixels: {} | Total frames: {}",
                fps_whole,
                fps_frac,
                frame_time.as_millis(),
                PIXELS_PER_FRAME,
                frame_count
            );

            frame_count = 0;
            last_fps_report = frame_end;
        }
    }
}
