use anyhow::Result;
use crate::{args, BUILD_DIRECTORY};

/// Writes an event to the event log file in compact JSON format.
pub fn write_event(event: &dyn Event) {
    use std::io::Write;
    
    let event_log_path = get_event_log_path().unwrap();
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&event_log_path)
        .unwrap();

    let event_json = event.to_json().unwrap();
    writeln!(file, "{}", event_json.replace("\n", "")).unwrap();
}

/// Writes start events for a test group and possibly a test round.
pub fn write_start_events() -> Result<TestGroupStartedEvent> {
    if is_start_of_test_round() {
        let round_started_event = TestRoundStartedEvent;
        write_event(&round_started_event);
    }

    let test_group_event = TestGroupStartedEvent::new();
    write_event(&test_group_event);

    Ok(test_group_event)
}

/// Writes end events for possibly a test round.
pub fn write_end_events(start_event: &TestGroupStartedEvent) -> Result<()> {
    if start_event.current_test_group + 1 >= start_event.total_test_groups {
        let round_ended_event = TestRoundEndedEvent;
        write_event(&round_ended_event);
    }

    Ok(())
}

/// Reads the event log to determine the current test group index.
pub fn get_current_test_group() -> usize {
    use std::io::BufRead;

    let event_log_path = get_event_log_path().unwrap();
    let file = std::fs::File::open(&event_log_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    for line in lines.iter().rev() {
        if line.contains("TestRoundEndedEvent") {
            return 0; // if previous round ended, start from first test group
        }

        if line.contains("TestGroupStartedEvent") {
            let json: serde_json::Value = serde_json::from_str(line).unwrap();
            let current_test_group = json.get("current_test_group").unwrap().as_u64().unwrap();
            return (current_test_group as usize) + 1;
        }
    }

    0 // default to first test group
}

/// Determines the total number of test groups based on the Cargo workspace structure.
pub fn get_total_test_groups() -> usize {
    let workspace_dir = args::get_workspace_root().unwrap();
    let manifest_toml_path = workspace_dir.join("Cargo.toml");
    let manifest_content = std::fs::read_to_string(manifest_toml_path).unwrap();
    let manifest: toml::Value = toml::from_str(&manifest_content).unwrap();

    let is_workspace = manifest.get("workspace").is_some();
    if is_workspace {
        let members = manifest
            .get("workspace")
            .and_then(|ws| ws.get("members"))
            .and_then(|m| m.as_array())
            .unwrap();

        members.len() + 1 // + 1 for the binary (main.rs)
    } else {
        2 // 1 for binary (main.rs), 1 for library (lib.rs)
    }
}

/// Determines if the current execution is the start of a new test round.
pub fn is_start_of_test_round() -> bool {
    use std::io::BufRead;

    let event_log_path = get_event_log_path().unwrap();
    let file = std::fs::File::open(&event_log_path).unwrap();
    let reader = std::io::BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();
    
    for line in lines.iter().rev() {
        if line.contains("TestRoundEndedEvent") { // previous round was ended, start of new round
            return true;
        }
        if line.contains("TestRoundStartedEvent") { // round already started
            return false;
        }
    }

    true // first round ever
}

/// Gets the path to the event log file, creating it if necessary.
fn get_event_log_path() -> Result<std::path::PathBuf> {
    let workspace_dir = args::get_workspace_root()?;
    let event_log_path = workspace_dir
        .join(BUILD_DIRECTORY)
        .join("event.log.json");

    std::fs::create_dir_all(event_log_path.parent().unwrap())?;
    if std::fs::metadata(&event_log_path).is_err() {
        std::fs::File::create(&event_log_path)?;
    }   

    Ok(event_log_path)
}

/// Trait representing a generic event with methods for type, timestamp, 
/// and JSON serialization.
pub trait Event {
    fn event_type(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn timestamp(&self) -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::json!({
            "event": self.event_type(),
            "timestamp": self.timestamp()
        }).to_string())
    }
}

/// Event indicating the start of a test round.
pub struct TestRoundStartedEvent;

impl Event for TestRoundStartedEvent {}

/// Event indicating the end of a test round.
pub struct TestRoundEndedEvent;

impl Event for TestRoundEndedEvent {}

/// Event indicating the start of a test group within a test round.
pub struct TestGroupStartedEvent {
    pub current_test_group: usize,
    pub total_test_groups: usize
}

impl TestGroupStartedEvent {
    pub fn new() -> Self {
        let current_test_group = get_current_test_group();
        let total_test_groups = get_total_test_groups();

        Self { current_test_group, total_test_groups }
    }
}

impl Event for TestGroupStartedEvent {
    fn to_json(&self) -> Result<String> {
        Ok(serde_json::json!({
            "event": self.event_type(),
            "timestamp": self.timestamp(),
            "current_test_group": self.current_test_group,
            "total_test_groups": self.total_test_groups
        }).to_string())
    }
}
