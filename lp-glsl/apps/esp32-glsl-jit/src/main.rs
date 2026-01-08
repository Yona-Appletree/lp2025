#![no_std]
#![no_main]

extern crate alloc;

use cranelift_codegen::isa::riscv32::isa_builder;
use cranelift_codegen::settings::{self, Configurable};
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::Instant;
use esp_hal::{clock::CpuClock, timer::systimer::SystemTimer};
use lp_glsl_compiler::Compiler;
use panic_rtt_target as _;
use target_lexicon::Triple;

// This creates a default app-descriptor required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Allocate heap - ESP32-C6 has plenty of RAM
    esp_alloc::heap_allocator!(size: 128 * 1024); // 128KB heap for Cranelift

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    // Initialize RTT after heap setup
    rtt_target::rtt_init_defmt!();

    info!("======================================");
    info!("ESP32-C6 GLSL JIT Test");
    info!("Testing Cranelift GLSL Compiler on Real RISC-V Hardware!");
    info!("======================================\n");

    // Fragment shader: pattern generator that takes pixel coordinates
    // This simulates real image rendering where each pixel is computed independently
    // Note: Using main() with parameters (non-standard GLSL, but supported by our compiler)
    let source = r#"
int main(int x, int y) {
    // Use pixel coordinates as seed for pattern generation
    // Scale coordinates to reasonable range for computation
    int seed_x = x * 10 + 100;
    int seed_y = y * 10 + 100;
    
    // Pattern generation through iterative computation
    // This is similar to what a real shader would do for effects like:
    // - Noise generation
    // - Pattern generation
    // - Simple raytracing
    int result = 0;
    int iterations = 50;  // Number of iterations for computation
    
    // Iterative pattern calculation (like a simplified mandelbrot/fractal)
    for (int i = 0; i < iterations; i = i + 1) {
        // Complex arithmetic operations
        int temp = seed_x * seed_x + seed_y * seed_y;
        result = result + (temp / 1000);
        
        // Update coordinates for next iteration
        int new_x = (seed_x * seed_x - seed_y * seed_y) / 100 + 200;
        int new_y = (2 * seed_x * seed_y) / 100 + 150;
        seed_x = new_x;
        seed_y = new_y;
        
        // Early exit if value gets too large (like escape condition)
        if (result > 10000) {
            break;
        }
    }
    
    // Normalize result to a reasonable range (0-999)
    result = result % 1000;
    
    return result;
}
"#;

    info!("======================================");
    info!("GLSL Shader Program:");
    info!("======================================");

    // Build the formatted program as a single string to output in chunks
    let lines: alloc::vec::Vec<&str> = source.lines().collect();
    let mut formatted_program = alloc::string::String::new();

    for (line_num, line) in lines.iter().enumerate() {
        let line_num_plus_one = line_num + 1;
        // Pad line number to 3 digits manually
        let padded_num = if line_num_plus_one < 10 {
            alloc::format!("  {}", line_num_plus_one)
        } else if line_num_plus_one < 100 {
            alloc::format!(" {}", line_num_plus_one)
        } else {
            alloc::format!("{}", line_num_plus_one)
        };
        formatted_program.push_str(&alloc::format!("{} | {}\n", padded_num, line));
    }

    // Output in larger chunks (every 10 lines) to avoid buffer issues
    let chunks: alloc::vec::Vec<&str> = formatted_program.lines().collect();
    info!("Program has {} lines total", chunks.len());

    for (chunk_idx, chunk) in chunks.chunks(10).enumerate() {
        let chunk_text = chunk.join("\n");
        info!(
            "Lines {} to {}:\n{}",
            chunk_idx * 10 + 1,
            core::cmp::min((chunk_idx + 1) * 10, chunks.len()),
            chunk_text.as_str()
        );
        // Longer delay between chunks to ensure serial buffer flushes
        embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
    }

    info!("======================================");
    info!("End of GLSL program ({} lines)", chunks.len());
    info!("");

    // Create RISC-V32 ISA
    info!("Step 1: Creating RISC-V32 ISA...");
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
            info!("  ✓ ISA created");
            isa
        }
        Err(_e) => {
            defmt::panic!("ISA creation failed");
        }
    };

    // Compile GLSL to machine code
    info!("Step 2: Compiling GLSL to RISC-V machine code...");
    let mut compiler = Compiler::new();
    let machine_code = match compiler.compile_to_code(source, &*isa) {
        Ok(code) => {
            info!("  ✓ Compilation successful: {} bytes", code.len());
            code
        }
        Err(e) => {
            // Log error details before panicking
            // Display error code and message (defmt supports {} for strings)
            info!("GLSL compilation failed!");
            info!("Error: {}", e.message.as_str());

            // Display location if available
            if let Some(ref loc) = e.location {
                info!("At line {}, column {}", loc.line, loc.column);
            }

            // Display span text if available (source code snippet)
            if let Some(ref span) = e.span_text {
                info!("Source code: {}", span.as_str());
            }

            // Display notes (these often contain detailed error information)
            // Break into smaller chunks to avoid defmt string length limits
            if !e.notes.is_empty() {
                info!("Error details:");
                for note in &e.notes {
                    // Split note by newlines and display each line
                    for line in note.lines() {
                        if !line.trim().is_empty() {
                            info!("  {}", line);
                        }
                    }
                }
            }

            defmt::panic!("GLSL compilation failed - see details above");
        }
    };

    info!("Step 3: Setting up continuous rendering loop...");

    // Ensure instruction cache coherency
    unsafe {
        core::arch::asm!("fence.i");
    }

    // Cast to function pointer - shader takes x, y coordinates and returns pixel value
    type ShaderFn = extern "C" fn(i32, i32) -> i32;
    let shader_fn: ShaderFn = unsafe { core::mem::transmute(machine_code.as_ptr()) };

    // Image dimensions: 64x64 pixels = 4096 pixels per frame
    const IMAGE_WIDTH: i32 = 64;
    const IMAGE_HEIGHT: i32 = 64;
    const PIXELS_PER_FRAME: i32 = IMAGE_WIDTH * IMAGE_HEIGHT;

    info!(
        "Rendering {}x{} image ({} pixels per frame)",
        IMAGE_WIDTH, IMAGE_HEIGHT, PIXELS_PER_FRAME
    );
    info!("Starting continuous rendering loop...\n");

    let mut frame_count: u32 = 0;
    let mut last_fps_report = Instant::now();
    const FPS_REPORT_INTERVAL_MS: u64 = 2000; // Report FPS every 2 seconds

    // Continuous rendering loop
    loop {
        // Render one frame (all pixels)
        let frame_start = Instant::now();

        // Render all pixels in the frame
        for y in 0..IMAGE_HEIGHT {
            for x in 0..IMAGE_WIDTH {
                let _pixel_value = shader_fn(x, y);
                // In a real implementation, we would store pixel_value in a framebuffer
            }
        }

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

            // Format FPS with 2 decimal places (defmt doesn't support .2 format)
            let fps_int = (fps * 100.0) as u32;
            let fps_whole = fps_int / 100;
            let fps_frac = fps_int % 100;

            info!(
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
