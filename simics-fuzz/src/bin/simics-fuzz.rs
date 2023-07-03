use anyhow::Result;
use clap::Parser;
use simics_fuzz::{args::Args, fuzzer::SimicsFuzzer};

pub fn main() -> Result<()> {
    let args = Args::parse();
    SimicsFuzzer::cli_main(args)
}
