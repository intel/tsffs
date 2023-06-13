use anyhow::Result;
use clap::Parser;
use simics_fuzz::args::Args;

pub fn main() -> Result<()> {
    let args = Args::parse();
    Ok(())
}
