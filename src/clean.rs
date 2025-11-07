use std::path::PathBuf;
use crate::{BUILD_DIRECTORY, KbootError};

pub fn clean() -> Result<(), KbootError> {
    let build_dir = PathBuf::from(BUILD_DIRECTORY);
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)
            .map_err(|e| KbootError::LoggerIoError(e, format!("Failed to clean build directory: {}", build_dir.display())))?;
        log::info!("Cleaned build directory: {}", build_dir.display());
    } else {
        log::info!("Build directory does not exist, nothing to clean: {}", build_dir.display());
    }

    let status = std::process::Command::new("cargo")
        .arg("clean")
        .status()
        .map_err(|e| KbootError::LoggerIoError(e, "Failed to execute cargo clean".to_string()))?;

    if status.success() {
        log::info!("Successfully ran 'cargo clean'");
    } else {
        log::warn!("'cargo clean' exited with a non-zero status: {}", status);
    }

    Ok(())
}
