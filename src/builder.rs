//! Builder for creating bootable disk images

use std::fs;
use anyhow::Result;
use bootloader::UefiBoot;
use crate::{args, BUILD_DIRECTORY};

/// Build a UEFI disk image (*.img) that contains the specified executable.
pub fn build_image(args: &Vec<String>) -> Result<()> {
    let manifest_directory = args::get_manifest_dir()?;
    let build_directory = manifest_directory.join(BUILD_DIRECTORY);
    let image_path = build_directory.join("kernel.img");
    let executable_path = args::get_executable(args)?;

    let mut config = bootloader::BootConfig::default();
    config.log_level = bootloader_boot_config::LevelFilter::Error;

    fs::create_dir_all(&build_directory)?;
    UefiBoot::new(&executable_path)
        .set_boot_config(&config)
        .create_disk_image(&image_path)?;

    Ok(())
}
