use anyhow::Result;

/// Entry point for the runner. This file should be kept as light as possible.
fn main() -> Result<()> {
    kboot::run()
}
