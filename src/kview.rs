use anyhow::Result;

use crate::{args, BUILD_DIRECTORY};

// note: called from `ktest` module
pub fn start_kview_if_needed(args: &Vec<String>) -> Result<()> {
    // check if there is a docker image for kview
    if is_docker_container_with_name_running("kview").unwrap_or(false) {
        log::info!("kview docker container is already running.");
        return Ok(());
    }

    let workspace_directory = args::get_workspace_root(&args)?;
    let build_path = workspace_directory.join(BUILD_DIRECTORY);

    log::info!("Starting kview docker container in detached mode...");
    let mut docker_binding = std::process::Command::new("docker");
    let command_builder = docker_binding
        .arg("run")                 // docker run command
        .arg("--rm")                // remove the container after it exits
        .arg("-d")                 // interactive terminal during runtime (works with kernel input)
        .args(["--name", "kview"])   // name of the container
        .args(["-p", "3000:3000"])  // port 3000 for web display (NextJS)
        // volumes (local filesystem -> container mappings)
        .args(["-v", &format!("{}:/kview", build_path.display())])
        // run kview in container
        .arg("philogroves/kview:0.1.1");

    let output = command_builder.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("Failed to start kview docker container: {}", stderr);
        return Err(anyhow::anyhow!("Failed to start kview docker container"));
    }
    
    // wait 5 seconds for the container to start
    std::thread::sleep(std::time::Duration::from_secs(5));

    log::info!("Opening kview in the default web browser at http://localhost:3000");
    webbrowser::open("http://localhost:3000")?;

    Ok(())
}

fn is_docker_container_with_name_running(name: &str) -> Result<bool> {
    // check if a docker container with the given name is running
    let output = std::process::Command::new("docker")
        .args(&["ps", "--filter", &format!("name={}", name), "--format", "{{.Names}}"])
        .output()?;

    let container_name = String::from_utf8_lossy(&output.stdout);
    Ok(container_name.lines().any(|line| line == name))
}