//! GLSL filetests infrastructure.
//!
//! This crate provides infrastructure for discovering, parsing, compiling, executing, and
//! verifying GLSL test files, matching Cranelift's filetests semantics.

#![deny(missing_docs)]

pub mod colors;
pub mod discovery;
pub mod output_mode;
pub mod parse;
pub mod runner;
pub mod test_compile;
pub mod test_run;
pub mod test_transform;
pub mod util;

use anyhow::Result;
use glob::{MatchOptions, glob_with};
use output_mode::OutputMode;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

/// Run a single filetest.
pub fn run_filetest(path: &Path) -> Result<()> {
    let (result, _stats) = run_filetest_with_line_filter(path, None, OutputMode::Detail)?;
    result
}

/// Count test cases in a file by counting `// run:` directives.
/// This works even if parsing fails later, so we can show stats.
pub(crate) fn count_test_cases(path: &Path, line_filter: Option<usize>) -> test_run::TestCaseStats {
    let mut stats = test_run::TestCaseStats::default();

    // Try to read and count run directives
    if let Ok(contents) = std::fs::read_to_string(path) {
        for (line_num, line) in contents.lines().enumerate() {
            let line_number = line_num + 1;

            // Apply line filter if provided
            if let Some(filter_line) = line_filter {
                if line_number != filter_line {
                    continue;
                }
            }

            // Check if this line contains a run directive
            if parse::parse_run::parse_run_directive_line(line).is_some() {
                stats.total += 1;
            }
        }
    }

    stats
}

/// Run a single filetest with optional line number filtering.
/// Returns the result and test case statistics.
pub fn run_filetest_with_line_filter(
    path: &Path,
    line_filter: Option<usize>,
    output_mode: OutputMode,
) -> Result<(Result<()>, test_run::TestCaseStats)> {
    // Count test cases early, even if parsing fails later
    let early_stats = count_test_cases(path, line_filter);

    let test_file = match parse::parse_test_file(path) {
        Ok(tf) => tf,
        Err(e) => {
            // Return error but preserve the test case count we already computed
            return Ok((Err(e), early_stats));
        }
    };

    // Validate line number if provided
    if let Some(line_number) = line_filter {
        let has_matching_directive = test_file
            .run_directives
            .iter()
            .any(|directive| directive.line_number == line_number);
        if !has_matching_directive {
            anyhow::bail!(
                "line {} does not contain a valid run directive",
                line_number
            );
        }
    }

    // Run compile test if requested
    // TODO: Implement compile test in Phase 4
    if test_file.test_types.contains(&parse::TestType::Compile) {
        // test_compile::run_compile_test(...)?;
    }

    // Run transform test if requested
    // TODO: Implement transform test in Phase 4
    if test_file
        .test_types
        .contains(&parse::TestType::TransformFixed32)
    {
        // test_transform::run_transform_fixed32_test(...)?;
    }

    // Run execution tests if requested
    if test_file
        .test_types
        .iter()
        .any(|t| matches!(t, parse::TestType::Run))
    {
        let (result, stats) =
            test_run::run_test_file_with_line_filter(&test_file, path, line_filter, output_mode)?;
        Ok((result, stats))
    } else {
        Ok((Ok(()), test_run::TestCaseStats::default()))
    }
}

/// Represents a parsed file path that may include a line number.
#[derive(Debug, Clone)]
struct FileSpec {
    path: PathBuf,
    line_number: Option<usize>,
}

/// Main entry point for `lp-test test`.
///
/// Take a list of filenames which can be either `.glsl` files or directories.
/// Files can optionally include line numbers in the format `file.glsl:42`.
/// Glob patterns are supported (e.g., `*.glsl`, `math/*`, `*add*`).
///
/// Files are interpreted as test cases and executed immediately.
///
/// Directories are scanned recursively for test cases ending in `.glsl`.
///
/// Mode is determined by test count:
/// - Single test (1 file): Full detailed output with all error information
/// - Multiple tests (>1 file): Minimal output with colored checkmarks
pub fn run(files: &[String]) -> anyhow::Result<()> {
    let filetests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("filetests");
    let mut test_specs = Vec::new();

    // Parse all file specifications, expanding glob patterns as needed
    for file_str in files {
        let specs = parse_file_spec_with_glob(file_str, &filetests_dir)?;

        for spec in specs {
            // Handle directories by recursively finding all .glsl files
            if spec.path.is_dir() {
                for entry in WalkDir::new(&spec.path) {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_file()
                                && path.extension().and_then(|s| s.to_str()) == Some("glsl")
                            {
                                test_specs.push(FileSpec {
                                    path: path.to_path_buf(),
                                    line_number: spec.line_number,
                                });
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: error walking directory {}: {}",
                                spec.path.display(),
                                e
                            );
                        }
                    }
                }
            } else if spec.path.is_file() {
                // Validate that the path exists and is a .glsl file
                if spec.path.extension().and_then(|s| s.to_str()) == Some("glsl") {
                    test_specs.push(spec);
                } else {
                    eprintln!(
                        "Warning: {} is not a .glsl file, skipping",
                        spec.path.display()
                    );
                }
            } else {
                eprintln!("Warning: {} does not exist, skipping", spec.path.display());
            }
        }
    }

    if test_specs.is_empty() {
        anyhow::bail!("no .glsl test files found");
    }

    // Sort for deterministic output
    test_specs.sort_by(|a, b| a.path.cmp(&b.path));

    println!("Running {} test file(s)...\n", test_specs.len());

    let start_time = Instant::now();
    let output_mode = OutputMode::from_test_count(test_specs.len());

    // Use sequential execution for single test, concurrent for multiple tests
    if test_specs.len() == 1 {
        // Single test: run sequentially and show full details
        let spec = &test_specs[0];
        let relative_path_str = relative_path(&spec.path, &filetests_dir);
        let display_path = if let Some(line) = spec.line_number {
            format!("{}:{}", relative_path_str, line)
        } else {
            relative_path_str
        };

        let (result, stats) = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_filetest_with_line_filter(&spec.path, spec.line_number, output_mode)
        })) {
            Ok(Ok((inner_result, inner_stats))) => (inner_result, inner_stats),
            Ok(Err(e)) => (Err(e), test_run::TestCaseStats::default()),
            Err(e) => {
                let panic_msg = if let Some(msg) = e.downcast_ref::<String>() {
                    msg.clone()
                } else if let Some(msg) = e.downcast_ref::<&'static str>() {
                    msg.to_string()
                } else {
                    format!("{:?}", e)
                };
                (
                    Err(anyhow::anyhow!("panicked: {}", panic_msg)),
                    test_run::TestCaseStats::default(),
                )
            }
        };

        match result {
            Ok(()) => {
                println!(
                    "{}",
                    colors::colorize(&format!("✓ {}", display_path), colors::GREEN)
                );
                let elapsed = start_time.elapsed();
                println!(
                    "\n{}",
                    format_results_summary(stats.passed, stats.failed, stats.total, 1, 0, elapsed)
                );
                return Ok(());
            }
            Err(_e) => {
                // Extract test expression and line number from error message
                let error_str = format!("{:#}", _e);
                let (test_expr, line_num) =
                    extract_test_info_from_error(&error_str, spec.line_number);
                let filename_only = Path::new(&display_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&display_path)
                    .to_string();
                let failure_line = if let Some(expr) = test_expr {
                    if let Some(line) = line_num {
                        format!("{}:{} {}", filename_only, line, expr)
                    } else {
                        format!("{} {}", filename_only, expr)
                    }
                } else {
                    filename_only
                };
                println!(
                    "{}",
                    colors::colorize(&format!("✗ {}", failure_line), colors::RED)
                );
                println!("\n{:#}", _e);
                let elapsed = start_time.elapsed();
                println!(
                    "\n{}",
                    format_results_summary(stats.passed, stats.failed, stats.total, 0, 1, elapsed)
                );
                anyhow::bail!("1 test file(s) failed");
            }
        }
    }

    // Multiple tests: use concurrent execution
    use runner::concurrent::ConcurrentRunner;

    #[derive(Debug)]
    enum TestState {
        New,
        Queued,
        Done(Result<()>),
    }

    struct TestEntry {
        spec: FileSpec,
        state: TestState,
        stats: test_run::TestCaseStats,
    }

    struct FailedTest {
        path: PathBuf,
        line_number: Option<usize>,
    }

    let mut tests: Vec<TestEntry> = test_specs
        .into_iter()
        .map(|spec| TestEntry {
            spec,
            state: TestState::New,
            stats: test_run::TestCaseStats::default(),
        })
        .collect();

    let mut concurrent_runner = ConcurrentRunner::new();
    let mut next_test = 0;
    let mut reported_tests = 0;
    let mut passed = 0;
    let mut failed = 0;
    let mut total_test_cases = 0;
    let mut passed_test_cases = 0;
    let mut failed_test_cases = 0;
    let mut failed_tests = Vec::new();

    // Queue all tests
    while next_test < tests.len() {
        let jobid = next_test;
        tests[jobid].state = TestState::Queued;
        concurrent_runner.put(
            jobid,
            &tests[jobid].spec.path,
            tests[jobid].spec.line_number,
            output_mode,
        );
        next_test += 1;
    }

    // Process replies and report results in order
    while reported_tests < tests.len() {
        // Check for completed jobs
        while let Some(reply) = concurrent_runner.try_get() {
            match reply {
                runner::concurrent::Reply::Done {
                    jobid,
                    result,
                    stats,
                } => {
                    tests[jobid].stats = stats;
                    tests[jobid].state = TestState::Done(result);
                }
            }
        }

        // Report next test in order if it's done
        if reported_tests < tests.len() {
            if let TestState::Done(ref result) = tests[reported_tests].state {
                let spec = &tests[reported_tests].spec;
                let relative_path_str = relative_path(&spec.path, &filetests_dir);
                let display_path = if let Some(line) = spec.line_number {
                    format!("{}:{}", relative_path_str, line)
                } else {
                    relative_path_str
                };

                let stats = &tests[reported_tests].stats;
                total_test_cases += stats.total;
                passed_test_cases += stats.passed;
                failed_test_cases += stats.failed;

                // Determine color for counts based on pass/fail ratio
                let counts_color = if stats.total > 0 {
                    if stats.passed == stats.total {
                        // All passed - green
                        colors::GREEN
                    } else if stats.passed > 0 {
                        // Some passed - yellow
                        colors::YELLOW
                    } else {
                        // All failed - red
                        colors::RED
                    }
                } else {
                    colors::GREEN // Default to green if no test cases
                };

                match result {
                    Ok(()) => {
                        // Multi-test mode: minimal output with colored checkmark, test case counts, and dimmed path
                        let status_marker = if colors::should_color() {
                            format!("{}{}{} ", colors::GREEN, "✓", colors::RESET)
                        } else {
                            "✓ ".to_string()
                        };
                        let counts_str = if stats.total > 0 {
                            let formatted = format!("{:2}/{:2}", stats.passed, stats.total);
                            if colors::should_color() {
                                format!("{}{}{}", counts_color, formatted, colors::RESET)
                            } else {
                                formatted
                            }
                        } else {
                            String::new()
                        };
                        let path_colored = if colors::should_color() {
                            format!("{}{} ", status_marker, counts_str)
                                + &format!("{}{}{}", colors::DIM, display_path, colors::RESET)
                        } else {
                            format!("{}{} {}", status_marker, counts_str, display_path)
                        };
                        println!("{}", path_colored);
                        // Flush stdout to ensure output appears immediately
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        passed += 1;
                    }
                    Err(_e) => {
                        // Multi-test mode: minimal output with colored X, test case counts, and dimmed path
                        // Error details (including panic messages) are suppressed in multi-test mode
                        let status_marker = if colors::should_color() {
                            format!("{}{}{} ", colors::RED, "✗", colors::RESET)
                        } else {
                            "✗ ".to_string()
                        };
                        let counts_str = if stats.total > 0 {
                            let formatted = format!("{:2}/{:2}", stats.passed, stats.total);
                            if colors::should_color() {
                                format!("{}{}{}", counts_color, formatted, colors::RESET)
                            } else {
                                formatted
                            }
                        } else {
                            String::new()
                        };
                        let path_colored = if colors::should_color() {
                            format!("{}{} ", status_marker, counts_str)
                                + &format!("{}{}{}", colors::DIM, display_path, colors::RESET)
                        } else {
                            format!("{}{} {}", status_marker, counts_str, display_path)
                        };
                        println!("{}", path_colored);
                        // Flush stdout to ensure output appears immediately
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        failed += 1;
                        failed_tests.push(FailedTest {
                            path: spec.path.clone(),
                            line_number: spec.line_number,
                        });
                    }
                }
                reported_tests += 1;
                continue;
            }
        }

        // If we can't report the next test yet, wait for more replies
        // But first check if any more replies are available without blocking
        // This prevents unnecessary blocking when multiple tests complete quickly
        let mut got_reply = false;
        while let Some(reply) = concurrent_runner.try_get() {
            got_reply = true;
            match reply {
                runner::concurrent::Reply::Done {
                    jobid,
                    result,
                    stats,
                } => {
                    tests[jobid].stats = stats;
                    tests[jobid].state = TestState::Done(result);
                }
            }
        }

        // Only block if we didn't get any replies and the next test isn't done
        if !got_reply {
            if let Some(reply) = concurrent_runner.get() {
                match reply {
                    runner::concurrent::Reply::Done {
                        jobid,
                        result,
                        stats,
                    } => {
                        tests[jobid].stats = stats;
                        tests[jobid].state = TestState::Done(result);
                    }
                }
            }
        }
    }

    // Shutdown threads
    concurrent_runner.shutdown();
    concurrent_runner.join();

    let elapsed = start_time.elapsed();

    // Print failed tests summary if there are failures
    if !failed_tests.is_empty() && !output_mode.show_full_output() {
        println!("\n{} Failed tests", failed_tests.len());
        println!("Run these commands to see test failure details\n");
        for failed_test in &failed_tests {
            let relative_path = relative_path(&failed_test.path, &filetests_dir);
            let test_path = if let Some(line) = failed_test.line_number {
                format!("{}:{}", relative_path, line)
            } else {
                relative_path
            };
            if colors::should_color() {
                println!(
                    "scripts/glsl-filetests.sh {}{}{}",
                    colors::DIM,
                    test_path,
                    colors::RESET
                );
            } else {
                println!("scripts/glsl-filetests.sh {}", test_path);
            }
        }
    }

    println!(
        "\n{}",
        format_results_summary(
            passed_test_cases,
            failed_test_cases,
            total_test_cases,
            passed,
            failed,
            elapsed
        )
    );

    if failed > 0 {
        anyhow::bail!("{} test file(s) failed", failed);
    }

    Ok(())
}

/// Check if a string contains glob pattern characters
fn contains_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

/// Expand glob patterns and return matching paths (files or directories)
fn expand_glob_patterns(pattern: &str, filetests_dir: &Path) -> Result<Vec<PathBuf>> {
    // Build the glob pattern - append pattern to filetests_dir
    // If pattern doesn't contain '/', it will match files/directories at the top level
    // If pattern contains '/', it will match at that specific path level
    let full_pattern = filetests_dir.join(pattern);

    // Convert to string for glob crate
    let pattern_str = full_pattern.to_string_lossy();

    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let mut matches = Vec::new();
    for entry in glob_with(&pattern_str, options)? {
        match entry {
            Ok(path) => {
                // Include both files and directories - directories will be handled later
                // to recursively find all .glsl files
                if path.is_file() {
                    // Only include .glsl files
                    if path.extension().and_then(|s| s.to_str()) == Some("glsl") {
                        matches.push(path);
                    }
                } else if path.is_dir() {
                    // Include directories - they'll be expanded to find all .glsl files
                    matches.push(path);
                }
            }
            Err(e) => {
                // Log warning but continue
                eprintln!("Warning: glob pattern error: {}", e);
            }
        }
    }

    // Sort for deterministic output
    matches.sort();
    Ok(matches)
}

/// Parse a file specification that may contain glob patterns and line numbers
fn parse_file_spec_with_glob(file_str: &str, filetests_dir: &Path) -> Result<Vec<FileSpec>> {
    // Check if it contains a line number (format: pattern:line_number)
    let (pattern, line_number) = if let Some(colon_pos) = file_str.find(':') {
        let (pattern_part, line_part) = file_str.split_at(colon_pos);
        let line_str = &line_part[1..]; // Skip the colon

        match line_str.parse::<usize>() {
            Ok(line) => (pattern_part, Some(line)),
            Err(_) => {
                // Not a valid line number, treat whole string as pattern
                (file_str, None)
            }
        }
    } else {
        (file_str, None)
    };

    // Check if pattern contains glob characters
    let paths = if contains_glob_pattern(pattern) {
        // Use glob to expand the pattern - this will match files and directories
        expand_glob_patterns(pattern, filetests_dir)?
    } else {
        // No glob characters - treat as literal path
        let full_path = filetests_dir.join(pattern);
        if full_path.exists() {
            vec![full_path]
        } else {
            vec![]
        }
    };

    // Create FileSpec for each matching path
    let mut specs = Vec::new();
    for path in paths {
        specs.push(FileSpec { path, line_number });
    }

    Ok(specs)
}

/// Compute relative path from filetests_dir to the given path.
fn relative_path(path: &Path, filetests_dir: &Path) -> String {
    path.strip_prefix(filetests_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// Extract test expression and line number from error message for failure marker display.
fn extract_test_info_from_error(
    error_str: &str,
    fallback_line: Option<usize>,
) -> (Option<String>, Option<usize>) {
    // Look for line number in rerun command (scripts/glsl-filetests.sh filename:line)
    let mut line_num = fallback_line;
    if let Some(script_pos) = error_str.find("scripts/glsl-filetests.sh") {
        if let Some(colon_pos) = error_str[script_pos..].find(':') {
            let after_colon = &error_str[script_pos + colon_pos + 1..];
            if let Some(end_pos) = after_colon.find(&['\n', ' '][..]) {
                if let Ok(num) = after_colon[..end_pos].trim().parse::<usize>() {
                    line_num = Some(num);
                }
            }
        }
    }

    // Look for // run: line in error message
    let test_expr = if let Some(run_start) = error_str.find("// run:") {
        if let Some(run_end) = error_str[run_start..].find('\n') {
            let run_line = &error_str[run_start + 7..run_start + run_end].trim();
            // Extract expression part (everything before == or ~=) and the expected value
            if let Some(op_pos) = run_line.rfind(" == ") {
                let expr = run_line[..op_pos].trim();
                let expected = run_line[op_pos + 4..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                Some(format!("{} == {}", expr, expected))
            } else if let Some(op_pos) = run_line.rfind(" ~= ") {
                let expr = run_line[..op_pos].trim();
                let expected = run_line[op_pos + 4..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                Some(format!("{} ~= {}", expr, expected))
            } else {
                Some(run_line.to_string())
            }
        } else {
            None
        }
    } else {
        None
    };

    (test_expr, line_num)
}

/// Format results summary with colors and timing.
fn format_results_summary(
    passed_test_cases: usize,
    failed_test_cases: usize,
    total_test_cases: usize,
    passed_files: usize,
    failed_files: usize,
    elapsed: std::time::Duration,
) -> String {
    let seconds = elapsed.as_secs_f64();
    let time_str = if seconds < 1.0 {
        format!("{:.0}ms", elapsed.as_millis())
    } else if seconds < 60.0 {
        format!("{:.2}s", seconds)
    } else {
        let mins = elapsed.as_secs() / 60;
        let remaining_secs = elapsed.as_secs_f64() - (mins * 60) as f64;
        format!("{}m {:.2}s", mins, remaining_secs)
    };

    if colors::should_color() {
        // Use red if there are failures, green if all passed
        let test_cases_color = if failed_test_cases > 0 {
            colors::RED
        } else {
            colors::GREEN
        };
        let files_color = if failed_files > 0 {
            colors::RED
        } else {
            colors::GREEN
        };

        let test_cases_part = if total_test_cases > 0 {
            format!(
                "{}{}/{} tests passed{}",
                test_cases_color,
                passed_test_cases,
                total_test_cases,
                colors::RESET
            )
        } else {
            String::new()
        };
        let files_part = format!(
            "{}{}/{} files passed{}",
            files_color,
            passed_files,
            passed_files + failed_files,
            colors::RESET
        );

        if total_test_cases > 0 {
            format!("{}, {} in {}", test_cases_part, files_part, time_str)
        } else {
            format!("{} in {}", files_part, time_str)
        }
    } else {
        if total_test_cases > 0 {
            format!(
                "{}/{} tests passed, {}/{} files passed in {}",
                passed_test_cases,
                total_test_cases,
                passed_files,
                passed_files + failed_files,
                time_str
            )
        } else {
            format!(
                "{}/{} files passed in {}",
                passed_files,
                passed_files + failed_files,
                time_str
            )
        }
    }
}
