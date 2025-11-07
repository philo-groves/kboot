use anyhow::Result;
use kboot::KbootError;

/// Entry point for the runner. This file should be kept as light as possible.
fn main() -> Result<(), KbootError> {
    kboot::run()
}
