# kboot

A simple Cargo target runner for usage with Rust-based operating system kernels, designed for usage with `ktest` for kernel testing.

This runner will package the given Rust binary into a bootable image file, and runs the kernel through a containerized version of QEMU. 

**x86_64 is the only currently supported architecture**

## Requirements
- A Rust-based kernel using the [bootloader](https://github.com/rust-osdev/bootloader) crate
- Docker

Nothing else! You do not even need QEMU to be installed; it will be managed via Docker.

## Features

- Creates a bootable disk image
- Runs the image in a Docker-based QEMU intance
- An event log for tracking state between test groups
- Restructures the line-by-line JSON from `ktest`:
  - Counts for pass/fail/ignore are calculated
  - Tests are grouped by module
- Test history is packaged by timestamp
- Automatically launches `kview` after testing
- Support for both UEFI and legacy BIOS images (use `--legacy-boot` if needed)

## Examples

Single Crate: https://github.com/philo-groves/example-kernel-kboot-ktest

Workspace: https://github.com/philo-groves/example-kernel-kboot-ktest-multicrate

## Usage

After being set up as a target runner, simple `cargo` commands may be used:

`cargo run`: Builds the image and launches QEMU in normal mode
`cargo test` or `cargo hack test --workspace`: Builds the image and launches QEMU in test mode

## Setup

In your .cargo/config.toml file, add the following:

```
[target.'cfg(target_os = "none")']
runner = "kboot"
```

If you are not using the `kboot` custom test framework, you may disable its postprocessing features with:

```
[target.'cfg(target_os = "none")']
runner = "kboot --no-ktest"
```

## Access

There are two primary interfaces due to the containerized QEMU instance:

### Command Line Interface

When QEMU executes (e.g. after `cargo run`), the Docker container is launched in "interactive" mode, which makes two-way communication possible through the same command line as `cargo`.

## Web Display

Similar to the native display of QEMU, a framebuffer may be drawn to: http://localhost:8006

Under the hood, this uses NoVNC to obtain a remote display connection to the containerized QEMU instance.
