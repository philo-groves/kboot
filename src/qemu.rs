use std::{path::PathBuf, time::Duration};
use anyhow::{anyhow, Result};
use crate::{args, BUILD_DIRECTORY, UUID};

/// Executes the QEMU virtual machine inside a Docker container, booting 
/// the UEFI image (*.img) that was built in the `build.rs` script.
/// 
/// The virtual machine is accessible through command line and web (noVNC)
/// interfaces. The web interface is available at `http://localhost:8006`
pub fn run() -> Result<Duration> {
    // check if docker is running, otherwise exit with error
    if !is_docker_running() {
        eprintln!("Docker does not seem to be running. Please start Docker and try again.");
        std::process::exit(1);
    }

    if args::has_qemu_options() {
        log::info!("QEMU options detected: {}", args::get_qemu_options()?.join(" "));
    }

    // prepare the arguments for running QEMU in Docker
    let mut run_args = RunArguments::default()?;

    // if the executable is a test executable, add the test arguments
    if args::is_test()? {
        run_args.qemu_test_args.extend(TEST_ARGUMENTS.iter().map(|s| s.to_string()));
        setup_test_output(&mut run_args)?;
    }

    // if custom QEMU arguments are provided, use them
    if args::has_qemu_options() {
        run_args.qemu_run_args = args::get_qemu_options()?;
    }

    run_args.print();

    // run QEMU in Docker and capture the exit code
    let mut stopwatch = stopwatch::Stopwatch::start_new();
    let exit_code = run_qemu(&run_args)?;
    stopwatch.stop();

    if exit_code == QemuExitCode::Failed as i32 {
        eprintln!("QEMU exited with failure code: {}", exit_code);
        std::process::exit(exit_code);
    } else if exit_code == QemuExitCode::Success as i32 {
        log::info!("QEMU exited successfully with code: {}", exit_code);
    } else {
        log::warn!("QEMU exited with unknown code: {}", exit_code);
    }

    Ok(stopwatch.elapsed())
}

/// A simple helper to determine if Docker daemon is running.
fn is_docker_running() -> bool {
    let output = std::process::Command::new("docker")
        .arg("info")
        .output();

    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Setup for the -debugcon output to a file
fn setup_test_output(run_args: &mut RunArguments) -> Result<()> {
    run_args.qemu_test_args.push("-debugcon".to_string());
    run_args.qemu_test_args.push(format!("file:/testing/logs/tests-{}.json", UUID.get().unwrap()));

    std::fs::create_dir_all(&run_args.testing_path)?;
    let log_path = run_args.testing_path.join(format!("tests-{}.json", UUID.get().unwrap()));
    std::fs::File::create(&log_path)?;

    Ok(())
}

/// Run QEMU inside a Docker container with the specified arguments.
fn run_qemu(run_args: &RunArguments)-> Result<i32> {
    // build the docker command to run the qemu image
    let mut docker_binding = std::process::Command::new("docker");
    let command_builder = docker_binding
        .arg("run")                 // docker run command
        .arg("--rm");               // remove the container after it exits
        
    #[cfg(not(feature = "ci"))]
    command_builder.arg("-it");     // interactive terminal during runtime (works with kernel input)

    command_builder.args(["--name", "qemu"])   // name of the container
        .args(["-p", "8006:8006"])  // port 8006 for web display (noVNC)
        // volumes (local filesystem -> container mappings)
        .args(["-v", &format!("{}/qemu-storage:/storage", run_args.build_path.display())])
        .args(["-v", &format!("{}:/boot.img", run_args.image_path.display())])
        .args(["-v", &format!("{}:/testing/logs", run_args.testing_path.display())])
        // kvm device is required for host communication from the qemu image
        .arg("--device=/dev/kvm")
        // network device and NET_ADMIN required for network bridge of qemu image
        .arg("--device=/dev/net/tun")
        .args(["--cap-add", "NET_ADMIN"])
        // QEMU arguments
        .arg("-e").arg(&format!("ARGUMENTS={} {}", run_args.qemu_run_args.join(" "), run_args.qemu_test_args.join(" ")))
        // run qemu in container using a specific version for stability, not latest
        .arg("qemux/qemu:7.12");

    // perform the execution of the run command and capture the exit code
    let exit_code = command_builder.status()?
        .code().ok_or_else(|| anyhow!("Failed to get exit code from QEMU process"))?;
    Ok(exit_code)
}

/// Arguments for QEMU when running tests.
const TEST_ARGUMENTS: [&str; 4] = [
    "-device", "isa-debug-exit,iobase=0xf4,iosize=0x04",
    "-display", "none"
    // -debugcon will be conditionally added for tests
];

/// A collection of arguments needed to run QEMU in Docker.
struct RunArguments {
    build_path: PathBuf,
    image_path: PathBuf,
    testing_path: PathBuf,
    qemu_run_args: Vec<String>,
    qemu_test_args: Vec<String>
}

impl RunArguments {
    /// Create default RunArguments based on the provided command line arguments.
    fn default() -> Result<Self> {
        let workspace_directory = args::get_workspace_root()?;
        let build_path = workspace_directory.join(BUILD_DIRECTORY);
        let image_path = build_path.join("kernel.img");
        let testing_path = build_path.join("testing");

        Ok(Self {
            build_path,
            image_path,
            testing_path,
            qemu_run_args: vec![],
            qemu_test_args: vec![]
        })
    }

    fn print(&self) {
        log::info!("=======================  <qemu>  =======================");
        log::info!("Build path:     {}", self.build_path.display());
        log::info!("Image path:     {}", self.image_path.display());
        log::info!("Testing path:   {}", self.testing_path.display());
        log::info!("QEMU run args:  {:?}", self.qemu_run_args);
        log::info!("QEMU test args: {:?}", self.qemu_test_args);
        log::info!("========================================================");
    }
}

/// Exit codes for QEMU. These codes are written to the I/O port `0xf4`
/// to signal QEMU to exit with the given code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11
}
