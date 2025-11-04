use anyhow::Result;
use crate::{args, BUILD_DIRECTORY};

const PORT: u16 = 3000;
const REMOTE_TAG: &str = "philogroves/kview:0.1.3";
const LOCAL_TAG: &str = "philogroves/kview_local:latest";

/// Start the kview docker container if it is not already running.
pub fn start_kview_if_needed() -> Result<()> {
    // check if there is a docker image for kview
    if is_docker_container_with_name_running("kview").unwrap_or(false) {
        log::info!("kview docker container is already running.");
        return Ok(());
    }

    // if port is in use, probably running outside of container already
    if is_already_running() {
        log::info!("Port {} is already in use, assuming kview is running.", PORT);
        return Ok(());
    }

    let workspace_directory = args::get_workspace_root()?;
    let build_path = workspace_directory.join(BUILD_DIRECTORY);

    let image_name = if cfg!(feature = "use_local_kview") {
        build_kview_image()?;
        LOCAL_TAG
    } else {
        REMOTE_TAG
    };

    log::info!("Starting kview docker container in detached mode...");
    let mut docker_binding = std::process::Command::new("docker");
    let command_builder = docker_binding
        .arg("run")
        .arg("--rm")
        .arg("-d")
        .args(["--name", "kview"])
        .args(["-p", "3000:3000"])
        .args(["-v", &format!("{}:/kview", build_path.display())])
        .arg(image_name);

    let output = command_builder.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Failed to start kview docker container: {}", stderr);
        return Err(anyhow::anyhow!("Failed to start kview docker container"));
    }

    log::info!("Opening kview in the default web browser at http://localhost:3000");
    webbrowser::open("http://localhost:3000")?;

    Ok(())
}

/// Check if a Docker container with the specified name is currently running.
fn is_docker_container_with_name_running(name: &str) -> Result<bool> {
    let output = std::process::Command::new("docker")
        .args(&["ps", "--filter", &format!("name={}", name), "--format", "{{.Names}}"])
        .output()?;

    let container_name = String::from_utf8_lossy(&output.stdout);
    Ok(container_name.lines().any(|line| line == name))
}

/// Check if kview is already running by attempting to connect to its web interface.
fn is_already_running() -> bool {
    match reqwest::blocking::get("http://localhost:3000") {
        Ok(_) => true,
        Err(_) => false
    }
}

/// Build the kview Docker image from the local kview directory.
fn build_kview_image() -> Result<()> {
    let workspace_directory = args::get_workspace_root()?;
    let kview_path = workspace_directory.parent().unwrap().join("kview");

    log::info!("Building kview docker image...");
    let mut docker_build = std::process::Command::new("docker");
    let command_builder = docker_build
        .arg("build")
        .args(["-t", LOCAL_TAG])
        .arg(".")
        .current_dir(&kview_path);

    let output = command_builder.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Failed to build kview docker image: {}", stderr);
        return Err(anyhow::anyhow!("Failed to build kview docker image"));
    }

    Ok(())
}