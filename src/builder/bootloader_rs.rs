use crate::{args, builder::{BuildError, BuilderArguments, BuilderBootloader, DiskImageType}};

pub struct BootloaderRsBootloader {

}

impl BuilderBootloader for BootloaderRsBootloader {
    fn create_disk_image(&self, builder_arguments: &BuilderArguments) -> Result<(), BuildError> {
        if builder_arguments.image_type == DiskImageType::Bios { // maybe a better way to do this?
            let mut builder_binding = bootloader::BiosBoot::new(&builder_arguments.executable_path);
            let mut bios_builder = builder_binding.set_boot_config(&builder_arguments.boot_config);

            if args::has_ramdisk() {
                let ramdisk_path = args::get_ramdisk_path().map_err(|_| BuildError::RamdiskPathInvalid)?;
                if let Some(path) = ramdisk_path {
                    bios_builder = bios_builder.set_ramdisk(&path);
                }
            }

            bios_builder.create_disk_image(&builder_arguments.image_path).unwrap();
        } else {
            let mut builder_binding = bootloader::UefiBoot::new(&builder_arguments.executable_path);
            let mut uefi_builder = builder_binding.set_boot_config(&builder_arguments.boot_config);

            if args::has_ramdisk() {
                let ramdisk_path = args::get_ramdisk_path().map_err(|_| BuildError::RamdiskPathInvalid)?;
                if let Some(path) = ramdisk_path {
                    uefi_builder = uefi_builder.set_ramdisk(&path);
                }
            }

            uefi_builder.create_disk_image(&builder_arguments.image_path).unwrap();
        }
        
        Ok(())
    }
}
