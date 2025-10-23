use std::{env, sync::OnceLock};
use anyhow::Result;
use uuid::Uuid;

mod builder;
mod args;
mod event;
mod ktest;
mod qemu;

/// Directory where build artifacts are stored
pub const BUILD_DIRECTORY: &str = ".build";

/// ID for tracking this session (e.g. unique file names)
pub static UUID: OnceLock<Uuid> = OnceLock::new();

/// Main entry point for the kboot runner.
pub fn run() -> Result<()> {
    UUID.set(Uuid::new_v4()).unwrap();
    let args: Vec<String> = env::args().collect();

    start_logger(&args)?;
    let start_event = event::write_start_events(&args)?;

    builder::build_image(&args)?;
    let run_duration = qemu::run(&args)?;

    if args::is_test(&args)? && !args::is_no_ktest(&args) {
        ktest::process_test_results(&args, run_duration)?;
    }

    event::write_end_events(&start_event, &args)?;
    Ok(())
}

/// Simple startup logs to display information about the executable
fn start_logger(args: &Vec<String>) -> Result<()> {
    let workspace_dir = args::get_workspace_root(&args)?;
    let log_file_path = workspace_dir.join(BUILD_DIRECTORY)
        .join("logs")
        .join(format!("kboot-{}.log", UUID.get().unwrap()));
    let file_spec = flexi_logger::FileSpec::default()
        .directory(log_file_path.parent().unwrap())
        .basename(log_file_path.file_stem().unwrap().to_str().unwrap())
        .suppress_timestamp();

    flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .log_to_file(file_spec)
        .append()
        .start()
        .unwrap();

    log::info!("Initiating kboot runner with arguments: {:?}", args);
    log::info!("====================  <executable>  ====================");
    log::info!("Executable path:             {}", args::get_executable(&args)?.display());
    log::info!("Executable parent directory: {}", args::get_executable_parent(&args)?.display());
    log::info!("Is executable a doctest?     {}", args::is_doctest(&args)?);
    log::info!("Is executable a test?        {}", args::is_test(&args)?);
    log::info!("Executable file stem:        {}", args::get_file_stem(&args)?);
    log::info!("Cargo manifest directory:    {}", args::get_manifest_dir()?.display());
    log::info!("Cargo.toml file path:        {}", args::get_manifest_toml()?.display());
    log::info!("Current working directory:   {}", args::get_workspace_root(&args)?.display());
    log::info!("========================================================");

    Ok(())
}
