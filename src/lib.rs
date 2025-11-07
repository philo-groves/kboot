use std::sync::OnceLock;
use anyhow::Result;
use uuid::Uuid;

mod builder;
mod args;
mod event;
mod ktest;
mod kview;
mod qemu;

/// Directory where build artifacts are stored
pub const BUILD_DIRECTORY: &str = ".build";

/// ID for tracking this session (e.g. unique file names)
pub static UUID: OnceLock<Uuid> = OnceLock::new();

/// Main entry point for the kboot runner.
pub fn run() -> Result<()> {
    UUID.set(Uuid::new_v4()).unwrap();

    start_logger()?;
    let start_event = event::write_start_events()?;

    builder::build_image().unwrap();
    let run_duration = qemu::run()?;

    if args::is_test()? && !args::is_no_ktest() {
        ktest::process_test_results(&start_event, run_duration)?;
    }

    event::write_end_events(&start_event)?;
    Ok(())
}

/// Simple startup logs to display information about the executable
fn start_logger() -> Result<()> {
    let log_file_path = get_log_file_path()?;

    if let Some(parent) = log_file_path.parent() && !parent.exists() {
        std::fs::create_dir_all(parent).unwrap();
    }

    simple_logging::log_to_file(log_file_path, log::LevelFilter::Info)?;

    log::info!("Initiating kboot runner with arguments: {:?}", args::get_arguments());
    log::info!("====================  <executable>  ====================");
    log::info!("Executable path:             {}", args::get_executable()?.display());
    log::info!("Executable parent directory: {}", args::get_executable_parent()?.display());
    log::info!("Is executable a doctest?     {}", args::is_doctest()?);
    log::info!("Is executable a test?        {}", args::is_test()?);
    log::info!("Executable file stem:        {}", args::get_file_stem()?);
    log::info!("Cargo manifest directory:    {}", args::get_manifest_dir()?.display());
    log::info!("Cargo.toml file path:        {}", args::get_manifest_toml()?.display());
    log::info!("Current working directory:   {}", args::get_workspace_root()?.display());
    log::info!("========================================================");

    Ok(())
}

fn get_log_file_path() -> Result<std::path::PathBuf> {
    let workspace_dir = args::get_workspace_root()?;
    let log_file_path = workspace_dir.join(BUILD_DIRECTORY)
        .join("logs")
        .join(format!("kboot-{}.log", UUID.get().unwrap()));
    Ok(log_file_path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::args::ARGUMENTS;
    use super::*;

    #[test]
    fn test_get_log_file_path() {
        let test_executable_path = PathBuf::from("target/debug/my_executable");
        let test_args = vec![
            "kboot".to_string(),
            test_executable_path.to_str().unwrap().to_string(),
        ];
        let _ = ARGUMENTS.set(test_args); // Ignore error if already set by another test
        let _ = UUID.set(Uuid::new_v4()); // Ignore error if already set by another test

        let path = get_log_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".build/logs/kboot-"));
        assert_eq!(path.extension().unwrap(), "log");
    }

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