use anyhow::{anyhow, Result};
use std::{env, path::PathBuf, sync::OnceLock};

// Command line arguments
static ARGUMENTS: OnceLock<Vec<String>> = OnceLock::new();

pub fn get_arguments() -> &'static Vec<String> {
    ARGUMENTS.get_or_init(|| env::args().collect())
}

/// Get the executable that should be packaged into an image and ran in QEMU
pub fn get_executable() -> Result<PathBuf> {
    let args = get_arguments();
    let args_without_options: Vec<&String> = args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .collect();

    // note: 0 is "kboot"
    if args_without_options.len() <= 1 {
        return Err(anyhow!("No executable specified"));
    }

    // executable is the last argument
    Ok(PathBuf::from(&args_without_options[args_without_options.len() - 1]))
}

/// Get the file stem of the executable that should be packaged 
/// into an image and ran in QEMU
pub fn get_file_stem() -> Result<String> {
    let exe = get_executable()?;
    let file_stem = exe.file_stem().ok_or_else(|| anyhow!("Executable has no file stem"))?;
    let file_stem_str = file_stem.to_str().ok_or_else(|| anyhow!("Executable file stem is not valid UTF-8"))?;
    
    Ok(file_stem_str.to_string())
}

/// Get the parent directory of the executable that should be 
/// packaged into an image and ran in QEMU
pub fn get_executable_parent() -> Result<PathBuf> {
    let exe = get_executable()?;
    let parent = exe.parent().ok_or_else(|| anyhow!("Executable has no parent directory"))?;
    let absolute_parent = std::path::absolute(parent)?;

    Ok(absolute_parent)
}

/// Get the workspace root directory by traversing up from the executable path 
/// until the "target" directory is found
pub fn get_workspace_root() -> Result<PathBuf> {
    let executable_binding = get_executable_parent()?;
    let mut executable_path = executable_binding.as_path();

    while let Some(parent) = executable_path.parent() {
        if parent.ends_with("target") {
            if let Some(workspace_root) = parent.parent() {
                let absolute_workspace_root = std::path::absolute(workspace_root)?;
                
                return Ok(absolute_workspace_root);
            } else {
                return Err(anyhow!("Workspace root not found"));
            }
        }
        executable_path = parent;
    }

    log::info!("Could not find 'target' directory in executable path, using executable's parent as workspace root");
    Ok(executable_path.to_path_buf())
}

/// Determine whether the executable is a Rust doctest executable
/// (i.e., its parent directory starts with "rustdoctest")
pub fn is_doctest() -> Result<bool> {
    return Ok(get_executable_parent()?
        .file_name()
        .ok_or_else(|| anyhow!("kernel executable's parent has no file name"))?
        .to_str()
        .ok_or_else(|| anyhow!("kernel executable's parent file name is not valid UTF-8"))?
        .starts_with("rustdoctest"))
}

/// Determine whether the executable is a Rust test executable
/// (i.e., its parent directory is "deps" or it is a doctest executable)
pub fn is_test() -> Result<bool> {
    let parent = get_executable_parent()?;

    Ok(is_doctest()? || parent.ends_with("deps"))
}

/// Get the Cargo manifest directory of the current project that is being run
pub fn get_manifest_dir() -> Result<PathBuf> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map_err(|_| anyhow!("CARGO_MANIFEST_DIR environment variable is not set"))?;
    
    Ok(manifest_dir.to_path_buf())
}

/// Get the Cargo.toml file location of the current project that is being run
pub fn get_manifest_toml() -> Result<PathBuf> {
    let manifest_dir = get_manifest_dir()?;
    let toml_path = manifest_dir.join("Cargo.toml");

    Ok(toml_path)
}

/// Determine whether ktest processing should be skipped
pub fn is_no_ktest() -> bool {
    let args = get_arguments();
    args.iter().any(|arg| arg == "--no-ktest")
}

/// Determine whether QEMU options have been provided
pub fn has_qemu_options() -> bool {
    let args = get_arguments();
    let has_qemu_arg = args.iter().any(|arg| arg == "--qemu");
    
    let mut in_quotes = false;
    for arg in args {
        if arg.starts_with('"') {
            in_quotes = true;
        }
        if in_quotes {
            if arg.ends_with('"') {
                in_quotes = false;
                break;
            }
        }
    }

    let has_qemu_options = !in_quotes;
    if has_qemu_arg && !has_qemu_options {
        panic!("--qemu must be followed by quoted QEMU options");
    }

    has_qemu_arg && has_qemu_options
}

/// Get the QEMU options provided after the `--qemu` flag
pub fn get_qemu_options() -> Result<Vec<String>> {
    let args = get_arguments();
    let qemu_index = args.iter().position(|arg| arg == "--qemu")
        .ok_or_else(|| anyhow!("--qemu not found in arguments"))?;
    
    let qemu_options = get_quoted_args(qemu_index + 1)
        .map_err(|_| anyhow!("--qemu must be followed by quoted QEMU options"))?;

    Ok(qemu_options)
}

pub fn is_legacy_boot() -> bool {
    let args = get_arguments();
    args.iter().any(|arg| arg == "--legacy-boot")
}

/// Determine whether a ramdisk path has been provided
pub fn has_ramdisk() -> bool {
    let args = get_arguments();
    args.iter().any(|arg| arg == "--ramdisk")
}

/// Get the ramdisk path provided after the `--ramdisk` flag
pub fn get_ramdisk_path() -> Result<Option<PathBuf>> {
    let args = get_arguments();
    let ramdisk_index = args.iter().position(|arg| arg == "--ramdisk");
    if let Some(index) = ramdisk_index {
        let ramdisk_args = get_quoted_args(index + 1)
            .map_err(|_| anyhow!("--ramdisk must be followed by a quoted path"))?;
        
        if ramdisk_args.len() != 1 {
            return Err(anyhow!("--ramdisk must be followed by exactly one path"));
        }

        return Ok(Some(PathBuf::from(&ramdisk_args[0])));
    }

    Ok(None)
}

/// Determine which bootloader to use based on command line arguments
pub fn get_bootloader_selection() -> BootloaderSelection {
    let args = get_arguments();
    if args.iter().any(|arg| arg == "--limine") {
        BootloaderSelection::Limine
    } else {
        BootloaderSelection::BootloaderCrate // default
    }
}

/// Get the limine.conf by scanning the project directory for it
pub fn get_limine_conf() -> Result<PathBuf> {
    let workspace_root = get_workspace_root()?;
    log::info!("Searching for limine.conf in workspace root: {:?}", workspace_root);

    if let Some(limine_conf) = scan_for_limine_conf(&workspace_root) {
        Ok(limine_conf)
    } else {
        Err(anyhow!("limine.conf not found in workspace"))
    }
}

/// Helper to recursively scan a directory for limine.conf file
fn scan_for_limine_conf(dir: &PathBuf) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        // First pass: check for limine.conf in current directory
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_file() {
                if let Some(file_name) = path.file_name() {
                    if file_name == "limine.conf" {
                        return Some(path);
                    }
                }
            }
        }
        
        // Second pass: recurse into subdirectories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if dir_name != "target" && dir_name != ".build" && !dir_name.starts_with('.') {
                            if let Some(found) = scan_for_limine_conf(&path) {
                                return Some(found);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub enum BootloaderSelection {
    BootloaderCrate,
    Limine,
}

/// Helper function to extract quoted arguments starting from a given index
fn get_quoted_args(start_index: usize) -> Result<Vec<String>> {
    let args = get_arguments();
    let mut combined = String::new();
    let mut in_quotes = false;

    for arg in &args[start_index..] {
        if arg.starts_with('"') {
            in_quotes = true;
        }
        if in_quotes {
            combined.push_str(arg);
            combined.push(' ');
        }
        if arg.ends_with('"') {
            break;
        }
    }

    if !in_quotes {
        return Err(anyhow!("Expected quoted arguments starting from index {}", start_index));
    }

    Ok(combined.trim().to_string()
        .split(" ")
        .map(|s| s.trim_start_matches("\"").trim_end_matches("\"").to_string())
        .collect::<Vec<String>>())
}
