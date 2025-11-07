use std::{io::Error, path::PathBuf, sync::OnceLock};
use anyhow::Result;
use uuid::Uuid;

mod builder;
mod args;
mod clean;
mod event;
mod ktest;
mod kview;
mod qemu;

/// Directory where build artifacts are stored
pub const BUILD_DIRECTORY: &str = ".build";

/// ID for tracking this session (e.g. unique file names)
pub static UUID: OnceLock<Uuid> = OnceLock::new();

/// Main entry point for the kboot runner.
pub fn run() -> Result<(), KbootError> {
    UUID.set(Uuid::new_v4()).unwrap();

    start_logger()?;
    let start_event = event::write_start_events()
        .map_err(|e| KbootError::EventFailedToWrite(format!("Failed to write start events: {}", e)))?;

    if args::should_clean() {
        return clean::clean();
    }

    builder::build_image().unwrap();
    let run_duration = qemu::run()
        .map_err(|e| KbootError::QemuFailedToRun(format!("Failed to run QEMU: {}", e)))?;

    if args::is_test().map_err(|_| KbootError::ArgumentFailedToParse("Failed to determine if executable is a test".to_string()))? && !args::is_no_ktest() {
        ktest::process_test_results(&start_event, run_duration)
            .map_err(|e| KbootError::EventFailedToWrite(format!("Failed to process ktest results: {}", e)))?;
    }

    event::write_end_events(&start_event).map_err(|e| KbootError::EventFailedToWrite(format!("Failed to write end events: {}", e)))?;
    Ok(())
}

/// Simple startup logs to display information about the executable
fn start_logger() -> Result<(), KbootError> {
    let log_file_path = get_log_file_path()?;
    if let Some(parent) = log_file_path.parent() && !parent.exists() {
        std::fs::create_dir_all(parent).map_err(|e| KbootError::LoggerIoError(e, "Failed to create log directory".to_string()))?;
    }
    
    simple_logging::log_to_file(log_file_path, log::LevelFilter::Info)
        .map_err(|e| KbootError::LoggerIoError(e, "Failed to initialize logger".to_string()))?;

    log::info!("Initiating kboot runner with arguments: {:?}", args::get_arguments());
    log::info!("====================  <executable>  ====================");
    log::info!("Clean mode:                  {:?}", args::should_clean());

    if args::should_clean() {
        // cut out early if in clean mode
        log::info!("========================================================");
        return Ok(());
    }

    log::info!("Executable path:             {}", args::get_executable().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get executable path".to_string()))?.display());
    log::info!("Executable parent directory: {}", args::get_executable_parent().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get executable parent directory".to_string()))?.display());
    log::info!("Is executable a doctest?     {}", args::is_doctest().map_err(|_| KbootError::ArgumentFailedToParse("Failed to determine if executable is a doctest".to_string()))?);
    log::info!("Is executable a test?        {}", args::is_test().map_err(|_| KbootError::ArgumentFailedToParse("Failed to determine if executable is a test".to_string()))?);
    log::info!("Executable file stem:        {}", args::get_file_stem().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get executable file stem".to_string()))?);
    log::info!("Cargo manifest directory:    {}", args::get_manifest_dir().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get cargo manifest directory".to_string()))?.display());
    log::info!("Cargo.toml file path:        {}", args::get_manifest_toml().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get Cargo.toml file path".to_string()))?.display());
    log::info!("Current working directory:   {}", args::get_workspace_root().map_err(|_| KbootError::ArgumentFailedToParse("Failed to get workspace root".to_string()))?.display());
    log::info!("========================================================");

    Ok(())
}

fn get_log_file_path() -> Result<std::path::PathBuf, KbootError> {
    let log_file_path = PathBuf::from(BUILD_DIRECTORY)
        .join("logs")
        .join(format!("kboot-{}.log", UUID.get().unwrap()));
    Ok(log_file_path)
}

#[derive(Debug)]
pub enum KbootError {
    /// Error indicating that the specified executable was not found.
    LoggerIoError(Error, String),
    QemuFailedToRun(String),
    ArgumentFailedToParse(String),
    EventFailedToWrite(String)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::args::ARGUMENTS;
    use super::*;

    #[test]
    pub fn test_simple_logging_initialization() {
        let test_executable_path = PathBuf::from("target/debug/my_executable");
        let test_args = vec![
            "kboot".to_string(),
            test_executable_path.to_str().unwrap().to_string(),
        ];
        let _ = ARGUMENTS.set(test_args); // Ignore error if already set by another test

        let _ = UUID.set(Uuid::new_v4()); // Ignore error if already set by another test

        let log_file_path = get_log_file_path().unwrap();

        // create parent directories if they don't exist
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }

        let result = simple_logging::log_to_file(log_file_path, log::LevelFilter::Info);
        // print error details, if any
        if let Err(e) = &result {
            eprintln!("Error initializing logger: {}", e);
        }

        assert!(result.is_ok());
    }
}
