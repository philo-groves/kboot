use std::{fs, path::PathBuf};
use anyhow::Result;
use bootloader::BootConfig;
use crate::{args::{self, BootloaderSelection}, BUILD_DIRECTORY};

pub mod disk;
pub mod bootloader_rs;
pub mod limine;

/// Build a legacy or UEFI disk image (*.img) that contains the specified executable.
pub fn build_image() -> Result<(), BuildError> {
    let builder_args = BuilderArguments::default().map_err(|_| BuildError::DirectoryCreationFailed)?;

    let mut config = bootloader::BootConfig::default();
    config.log_level = bootloader_boot_config::LevelFilter::Error;

    fs::create_dir_all(&builder_args.build_directory).map_err(|_| BuildError::DirectoryCreationFailed)?;

    let bootloader: Box<dyn BuilderBootloader> = match args::get_bootloader_selection() {
        BootloaderSelection::BootloaderCrate => Box::new(bootloader_rs::BootloaderRsBootloader {}),
        BootloaderSelection::Limine => Box::new(limine::LimineBootloader {}),
    };
    bootloader.create_disk_image(&builder_args)?;

    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DiskImageType {
    Uefi,
    Bios
}

pub trait BuilderBootloader {
    fn create_disk_image(&self, builder_arguments: &BuilderArguments) -> Result<(), BuildError>;
}

pub struct BuilderArguments {
    pub executable_path: PathBuf,
    pub build_directory: PathBuf,
    pub image_path: PathBuf,
    pub boot_config: BootConfig,
    pub image_type: DiskImageType
}

impl BuilderArguments {
    fn default() -> Result<Self> {
        let workspace_directory = args::get_workspace_root()?;
        let build_directory = workspace_directory.join(BUILD_DIRECTORY);
        let image_path = build_directory.join("kernel.img");
        let executable_path = args::get_executable()?;
        let boot_config = BootConfig::default();

        let image_type = if args::is_legacy_boot() {
            DiskImageType::Bios
        } else {
            DiskImageType::Uefi
        };

        Ok(Self {
            executable_path,
            build_directory,
            image_path,
            boot_config,
            image_type
        })
    }
}

#[derive(Debug)]
pub enum BuildError {
    DirectoryCreationFailed,
    CloneLimineBinaryFailed,
    RamdiskPathInvalid,
    LimineConfNotFound,
    DirectoryReadFailed,
    PathPrefixFailed
}