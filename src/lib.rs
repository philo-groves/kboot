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
    let workspace_dir = args::get_workspace_root()?;
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
