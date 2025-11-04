use std::fs;
use anyhow::Result;
use bootloader::{BiosBoot, UefiBoot};
use crate::{args, BUILD_DIRECTORY};

/// Build a legacy or UEFI disk image (*.img) that contains the specified executable.
pub fn build_image() -> Result<()> {
    let workspace_directory = args::get_workspace_root()?;
    let build_directory = workspace_directory.join(BUILD_DIRECTORY);
    let image_path = build_directory.join("kernel.img");
    let executable_path = args::get_executable()?;

    let mut config = bootloader::BootConfig::default();
    config.log_level = bootloader_boot_config::LevelFilter::Error;

    fs::create_dir_all(&build_directory)?;

    let image_type = if args::is_legacy_boot() {
        DiskImageType::Bios
    } else {
        DiskImageType::Uefi
    };

    if image_type == DiskImageType::Bios { // maybe a better way to do this?
        let mut builder_binding = BiosBoot::new(&executable_path);
        let mut bios_builder = builder_binding
            .set_boot_config(&config);

        if args::has_ramdisk() {
            let ramdisk_path = args::get_ramdisk_path()?;
            if let Some(path) = ramdisk_path {
                bios_builder = bios_builder.set_ramdisk(&path);
            }
        }

        bios_builder.create_disk_image(&image_path)?;
    } else {
        let mut builder_binding = UefiBoot::new(&executable_path);
        let mut uefi_builder = builder_binding
            .set_boot_config(&config);

        if args::has_ramdisk() {
            let ramdisk_path = args::get_ramdisk_path()?;
            if let Some(path) = ramdisk_path {
                uefi_builder = uefi_builder.set_ramdisk(&path);
            }
        }

        uefi_builder.create_disk_image(&image_path)?;
    }


    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DiskImageType {
    Uefi,
    Bios
}