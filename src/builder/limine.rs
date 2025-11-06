use std::{collections::BTreeMap, fs::{self, read_dir}, path::Path, process::Command};
use crate::{args::{self, get_workspace_root}, builder::{disk::{file_data_source::FileDataSource, gpt}, BuildError, BuilderArguments, BuilderBootloader, DiskImageType}};

pub struct LimineBootloader;

impl BuilderBootloader for LimineBootloader {
    fn create_disk_image(&self, builder_args: &BuilderArguments) -> Result<(), BuildError> {
        if builder_args.image_type == DiskImageType::Bios {
            panic!("Limine bootloader does not support BIOS booting (UEFI only).");
        }

        setup_limine_root(&builder_args)?;
        clone_limine_repo(&builder_args)?;
        setup_limine_conf(&builder_args)?;
        setup_limine_bios(&builder_args)?;

        build_limine_image(&builder_args)
    }
}

fn setup_limine_root(builder_args: &BuilderArguments) -> Result<(), BuildError> {
    log::info!("Setting up Limine ISO root directory...");
    
    let limine_root = builder_args.build_directory.join("iso_root");

    if limine_root.exists() {
        fs::remove_dir_all(&limine_root).map_err(|_| BuildError::DirectoryCreationFailed)?;
    }

    fs::create_dir_all(&limine_root).map_err(|_| BuildError::DirectoryCreationFailed)
}

fn clone_limine_repo(builder_args: &BuilderArguments) -> Result<(), BuildError> {
    const BRANCH: &str = "v10.x-binary";
    const URL: &str = "https://github.com/limine-bootloader/limine.git";

    let path = builder_args.build_directory.join("limine");

    if path.exists() {
        log::info!("Limine repository already cloned, skipping clone step...");
        return Ok(());
    }

    log::info!("Cloning Limine repository from {} (branch: {})...", URL, BRANCH);

    let repo = match git2::build::RepoBuilder::new().branch(BRANCH).clone(URL, &path) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    let head = repo.head().map_err(|_| BuildError::CloneLimineBinaryFailed)?;
    let head_id = head.target().unwrap();
    let head_commit = repo.find_commit(head_id).map_err(|_| BuildError::CloneLimineBinaryFailed)?;

    log::info!("Cloned Limine repository at commit {}", head_commit.id());
    Ok(())
}

fn setup_limine_conf(builder_args: &BuilderArguments) -> Result<(), BuildError> {
    log::info!("Setting up limine.conf for Limine...");

    let limine_conf_src = args::get_limine_conf().map_err(|_| BuildError::LimineConfNotFound)?;
    let limine_conf_dst = builder_args.build_directory.join("iso_root").join("boot").join("limine").join("limine.conf");

    fs::create_dir_all(limine_conf_dst.parent().unwrap()).unwrap();
    fs::copy(limine_conf_src, limine_conf_dst).unwrap();

    Ok(())
}

fn setup_limine_bios(builder_args: &BuilderArguments) -> Result<(), BuildError> {
    log::info!("Setting up Limine BIOS and EFI files...");

    const BIOS_FILES : [&str; 3] = [
        "limine-bios.sys",
        "limine-bios-cd.bin",
        "limine-uefi-cd.bin"
    ];

    fs::create_dir_all(builder_args.build_directory.join("iso_root").join("boot").join("limine")).map_err(|_| BuildError::DirectoryCreationFailed)?;
    for file in BIOS_FILES.iter() {
        let src = builder_args.build_directory.join("limine").join(file);
        let dst = builder_args.build_directory.join("iso_root").join("boot").join("limine").join(file);

        fs::copy(src, dst).unwrap();
    }

    const EFI_FILES : [&str; 2] = [
        "BOOTX64.EFI",
        "BOOTIA32.EFI"
    ];

    fs::create_dir_all(builder_args.build_directory.join("iso_root").join("EFI").join("BOOT")).map_err(|_| BuildError::DirectoryCreationFailed)?;
    for file in EFI_FILES.iter() {
        let src = builder_args.build_directory.join("limine").join(file);
        let dst = builder_args.build_directory.join("iso_root").join("EFI").join("BOOT").join(file);

        fs::copy(src, dst).unwrap();
    }

    Ok(())
}

fn build_limine_image(builder_args: &BuilderArguments) -> Result<(), BuildError> {
    let executable_src = &builder_args.executable_path;
    let executable_dst = builder_args.build_directory.join("iso_root").join("boot").join("kernel").join("kernel");

    fs::create_dir_all(executable_dst.parent().unwrap()).map_err(|_| BuildError::DirectoryCreationFailed)?;
    fs::copy(executable_src, executable_dst).unwrap();

    let iso_root = builder_args.build_directory.join("iso_root");
    let output_image = builder_args.build_directory.join("kernel.img");
    
    log::info!("Creating disk image at {:?}", output_image);

    let mut internal_files = BTreeMap::new();
    let mut dirs_to_process = vec![iso_root.clone()];
    while let Some(current_dir) = dirs_to_process.pop() {
        for entry in read_dir(&current_dir).map_err(|_| BuildError::DirectoryReadFailed)? {
            let entry = entry.map_err(|_| BuildError::DirectoryReadFailed)?;
            let path = entry.path();
            let relative_path = path.strip_prefix(&iso_root).map_err(|_| BuildError::PathPrefixFailed)?;

            if path.is_dir() {
                dirs_to_process.push(path);
            } else if path.is_file() {
                log::info!("Adding file to disk image: {:?}", relative_path);
                internal_files.insert(relative_path.to_string_lossy().to_string(), FileDataSource::File(path));
            }
        }
    }

    let fat_partition = crate::builder::disk::fat::create_fat_filesystem_image(BTreeMap::new(), internal_files).unwrap();
    gpt::create_gpt_disk(&fat_partition.path(), output_image.as_path()).unwrap();
    // let fat_partition_path = fat_partition.path().to_path_buf();

    // log::info!("Copying FAT from {:?} to {:?}", fat_partition_path, output_image);
    // fs::copy(&fat_partition_path, &output_image).unwrap();
    
    // Install Limine bootloader
    install_limine(&output_image).unwrap();

    fat_partition
        .close().unwrap();
    
    Ok(())
}

fn install_limine(disk_image: &Path) -> std::io::Result<()> {
    let is_windows = cfg!(target_os = "windows");
    let limine_executable = if is_windows {
        "limine.exe"
    } else {
        "limine"
    };
    let limine_path = get_workspace_root().unwrap().join(".build").join("limine").join(limine_executable);
    log::info!("Installing Limine bootloader using binary at {}", limine_path.display());

    // use sh to execute limine command on non-windows platforms
    let output = if is_windows {
        Command::new(format!("{}", limine_path.display()))
            .arg("bios-install")
            .arg(disk_image)
            .output()?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!("{} bios-install {}", limine_path.display(), disk_image.display()))
            .output()?
    };

    // Install Limine to MBR
    // let output = Command::new("limine")
    //     .arg("bios-install")
    //     .arg(disk_image)
    //     .output()?;
    
    if !output.status.success() {
        eprintln!("Limine installation failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Limine installation failed"
        ));
    }
    
    println!("Limine bootloader installed successfully!");
    Ok(())
}
