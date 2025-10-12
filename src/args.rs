use anyhow::{anyhow, Result};
use std::{env, path::PathBuf};

/// Get the executable that should be packaged into an image 
/// and ran in QEMU
pub fn get_executable(args: &Vec<String>) -> Result<PathBuf> {
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
pub fn get_file_stem(args: &Vec<String>) -> Result<String> {
    let exe = get_executable(args)?;
    let file_stem = exe.file_stem().ok_or_else(|| anyhow!("Executable has no file stem"))?;
    let file_stem_str = file_stem.to_str().ok_or_else(|| anyhow!("Executable file stem is not valid UTF-8"))?;
    
    Ok(file_stem_str.to_string())
}

/// Get the parent directory of the executable that should be 
/// packaged into an image and ran in QEMU
pub fn get_executable_parent(args: &Vec<String>) -> Result<PathBuf> {
    let exe = get_executable(args)?;
    let parent = exe.parent().ok_or_else(|| anyhow!("Executable has no parent directory"))?;
    
    Ok(parent.to_path_buf())
}

/// Determine whether the executable is a Rust doctest executable
/// (i.e., its parent directory starts with "rustdoctest")
pub fn is_doctest(args: &Vec<String>) -> Result<bool> {
    return Ok(get_executable_parent(args)?
        .file_name()
        .ok_or_else(|| anyhow!("kernel executable's parent has no file name"))?
        .to_str()
        .ok_or_else(|| anyhow!("kernel executable's parent file name is not valid UTF-8"))?
        .starts_with("rustdoctest"))
}

/// Determine whether the executable is a Rust test executable
/// (i.e., its parent directory is "deps" or it is a doctest executable)
pub fn is_test(args: &Vec<String>) -> Result<bool> {
    let parent = get_executable_parent(args)?;

    Ok(is_doctest(args)? || parent.ends_with("deps"))
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

pub fn is_no_ktest(args: &Vec<String>) -> bool {
    args.iter().any(|arg| arg == "--no-ktest")
}
