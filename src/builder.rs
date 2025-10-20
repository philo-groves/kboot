//! Builder for creating bootable disk images

use std::fs;
use anyhow::Result;
use bootloader::UefiBoot;
use crate::{args, BUILD_DIRECTORY};

/// Build a UEFI disk image (*.img) that contains the specified executable.
pub fn build_image(args: &Vec<String>) -> Result<()> {
    let workspace_directory = args::get_workspace_root(&args)?;
    let build_directory = workspace_directory.join(BUILD_DIRECTORY);
    let image_path = build_directory.join("kernel.img");
    let executable_path = args::get_executable(args)?;

    let mut config = bootloader::BootConfig::default();
    config.log_level = bootloader_boot_config::LevelFilter::Error;

    fs::create_dir_all(&build_directory)?;
    let mut builder_binding = UefiBoot::new(&executable_path);
    let mut builder = builder_binding
        .set_boot_config(&config);

    if args::has_ramdisk(args) {
        let ramdisk_path = args::get_ramdisk_path(args)?;
        if let Some(path) = ramdisk_path {
            builder = builder.set_ramdisk(&path);
        }
    }

    builder
        .create_disk_image(&image_path)?;

    Ok(())
}
