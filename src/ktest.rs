use std::{fs, io, sync::{OnceLock, RwLock}, time::Duration};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use crate::{args, event::TestGroupStartedEvent, kview, BUILD_DIRECTORY};

/// A global, thread-safe storage for the test group being processed.
static USE_KVIEW: OnceLock<RwLock<bool>> = OnceLock::new();

/// A global, thread-safe storage for the test group being processed.
static TEST_GROUP: OnceLock<RwLock<TestGroup>> = OnceLock::new();

/// Tests from `'ktest` are delivered through the -debugcon device
/// in a line-by-line fashion. Each line is a JSON object that
/// describes a test group, test result, or related object.
/// 
/// This function collects those lines and uses the power of 
/// the standard library to parse them into structured data.
pub fn process_test_results(args: &Vec<String>, start_event: &TestGroupStartedEvent, run_duration: Duration) -> Result<()> {
    if !args::is_test(args)? { // ignore this for `cargo run` etc
        return Ok(());
    }

    let workspace_dir = args::get_workspace_root(&args)?;
    let qemu_output_path = workspace_dir.join(BUILD_DIRECTORY)
        .join("testing")
        .join(format!("tests-{}.json", crate::UUID.get().unwrap()));

    if !qemu_output_path.exists() { // if nothing exists, nothing to process
        return Ok(());
    }

    let qemu_outputfile = fs::File::open(&qemu_output_path)?;
    let reader = io::BufReader::new(qemu_outputfile);

    log::info!("====================  <test results>  ====================");
    for line_result in reader.lines() {
        let line = line_result?; 
        log::info!("{}", line);
        process_json_line(&line, run_duration)?;
    }

    process_summary()?;

    let test_group = TEST_GROUP.get()
        .ok_or_else(|| anyhow!("No test group found after processing test results"))?
        .read()
        .map_err(|_| anyhow!("Failed to acquire read lock on test group"))?;
    let test_output_path = workspace_dir.join(BUILD_DIRECTORY)
        .join("testing")
        .join(format!("tests-{}.json", test_group.test_group));
    let test_output_file = fs::File::create(&test_output_path)?;

    serde_json::to_writer_pretty(&test_output_file, &*test_group)?;
    fs::remove_file(&qemu_output_path)?;

    let is_final_group = start_event.current_test_group + 1 >= start_event.total_test_groups;
    if is_final_group {
        process_final_json(args)?;
        let use_kview = USE_KVIEW.get()
            .ok_or_else(|| anyhow!("No use_kview flag found after processing test results"))?
            .read()
            .map_err(|_| anyhow!("Failed to acquire read lock on use_kview"))?;

        if use_kview.clone() {
            kview::start_kview_if_needed(args)?;
        }
    }

    Ok(())
}

/// Process a single line of JSON input from the test output. This function 
/// updates the global TEST_GROUP state as needed. If a line contained a test 
/// result, it is added to the appropriate module within the test group.
fn process_json_line(line: &str, run_duration: Duration) -> Result<()> {
    let json: serde_json::Value = serde_json::from_str(line)?;
    
    if json.get("test_group").is_some() {
        let test_group = process_test_group_json(&json, run_duration)?;
        TEST_GROUP.set(RwLock::new(test_group.0))
            .map_err(|_| anyhow!("Test group already set"))?;
        USE_KVIEW.set(RwLock::new(test_group.1))
            .map_err(|_| anyhow!("Use kview already set"))?;
    } else if json.get("test").is_some() {
        let mut test = process_test_json(&json)?;
        let mut test_group = TEST_GROUP.get()
            .ok_or_else(|| anyhow!("Test group not set before test result"))?
            .write()
            .map_err(|_| anyhow!("Failed to acquire write lock on test group"))?;
        
        let module_name = module_from_name(&test.test);
        test.test = function_from_name(&test.test);

        if let Some(module) = test_group.modules.iter_mut().find(|m| m.module == module_name) {
            module.tests.push(test);
        } else {
            test_group.modules.push(TestModule {
                module: module_name,
                tests: vec![test],
            });
        }
    }

    Ok(())
}

/// Process a JSON object representing a test group and return a TestGroup struct.
fn process_test_group_json(json: &serde_json::Value, run_duration: Duration) -> Result<(TestGroup, bool)> {
    let name = json.get("test_group")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("test_group field is missing or not a string"))?
        .to_string();
    let test_count = json.get("test_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("test_count field is missing or not a u64"))?;
    let use_kview = json.get("use_kview")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let summary = TestSummary {
        total: test_count,
        passed: 0,
        failed: 0,
        ignored: 0,
        duration: run_duration.as_millis() as u64
    };

    Ok((TestGroup {
        test_group: name,
        summary,
        modules: Vec::new()
    }, use_kview))
}

/// Process a JSON object representing a test result and return a TestResult struct.
fn process_test_json(json: &serde_json::Value) -> Result<TestResult> {
    let test = json.get("test")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("test field is missing or not a string"))?
        .to_string();
    let result = json.get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("result field is missing or not a string"))?
        .to_string();
    let cycle_count = json.get("cycle_count")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow!("cycle_count field is missing or not a u64"))?;

    let location = json.get("location")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let message = json.get("message")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(TestResult {
        test,
        result,
        cycle_count,
        location,
        message
    })
}

/// After all test results have been processed, this function computes
/// the summary statistics (passed, failed, missed) for the test group.
/// 
/// It updates the global TEST_GROUP state accordingly.
fn process_summary() -> Result<()> {
    let mut test_group = TEST_GROUP.get()
        .ok_or_else(|| anyhow!("No test group found for summary processing"))?
        .write()
        .map_err(|_| anyhow!("Failed to acquire write lock on test group"))?;

    test_group.summary.passed = test_group.modules.iter()
        .map(|m| m.tests.iter().filter(|t| t.result == "pass").count() as u64)
        .sum();
    test_group.summary.failed = test_group.modules.iter()
        .map(|m| m.tests.iter().filter(|t| t.result == "fail").count() as u64)
        .sum();
    test_group.summary.ignored = test_group.summary.total
        .saturating_sub(test_group.summary.passed + test_group.summary.failed);
    
    Ok(())
}

fn process_final_json(args: &Vec<String>) -> Result<()> {
    let workspace_dir = args::get_workspace_root(&args)?;
    let testing_dir = workspace_dir.join(BUILD_DIRECTORY).join("testing");
    let current_time_millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis();
    let timestamped_testing_dir = workspace_dir.join(BUILD_DIRECTORY).join(format!("testing-{}", current_time_millis));

    // create timestamped directory and move all JSON files there
    fs::create_dir_all(&timestamped_testing_dir)?;
    for entry in fs::read_dir(&testing_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file_name = path.file_name().ok_or_else(|| anyhow!("Failed to get file name"))?;
            let dest_path = timestamped_testing_dir.join(file_name);
            fs::rename(&path, &dest_path)?;
        }
    }
    fs::remove_dir_all(&testing_dir)?;

    Ok(())
}

/// Helper function to extract the module name from a fully qualified test name.
fn module_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.rsplitn(2, "::").collect();
    if parts.len() == 2 {
        parts[1].to_string()
    } else {
        "unknown".to_string()
    }
}

/// Helper function to extract the function name from a fully qualified test name.
fn function_from_name(name: &str) -> String {
    let parts: Vec<&str> = name.rsplitn(2, "::").collect();
    if parts.len() == 2 {
        parts[0].to_string()
    } else {
        name.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TestGroup {
    test_group: String,
    summary: TestSummary,
    modules: Vec<TestModule>
}

#[derive(Serialize, Deserialize, Debug)]
struct TestSummary {
    total: u64,
    passed: u64,
    failed: u64,
    ignored: u64,
    duration: u64
}

#[derive(Serialize, Deserialize, Debug)]
struct TestModule {
    module: String,
    tests: Vec<TestResult>
}

#[derive(Serialize, Deserialize, Debug)]
struct TestResult {
    test: String,
    result: String,
    cycle_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<String>, // failure only
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>   // failure only
}
