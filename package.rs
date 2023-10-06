#!/usr/bin/env -S cargo +nightly -Z script

//! ```cargo
//! [dependencies]
//! anyhow = "*"
//! clap = { version = "*", features = ["derive"] }
//! serde_json = "*"
//! walkdir = "*"
//! ```

use anyhow::{anyhow, Result};
use clap::Parser;
use serde_json::to_string;
use std::{env::set_current_dir, fs::write, path::PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug, Clone)]
struct Args {
    #[clap(short = 'o', long)]
    output: PathBuf,
    #[clap(short = 'f', long)]
    file: Vec<PathBuf>,
    #[clap(short = 'd', long)]
    directory: Vec<PathBuf>,
    #[clap(short = 'w', long)]
    working_directory: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    set_current_dir(&args.working_directory).map_err(|e| {
        anyhow!(
            "Failed to set current directory to {}: {e}",
            args.working_directory.display()
        )
    })?;

    let mut files = args.file.clone();

    args.directory.iter().for_each(|d| {
        WalkDir::new(&d)
            .into_iter()
            .filter_map(|p| p.ok())
            .map(|p| p.path().to_path_buf())
            .filter(|p| p.is_file())
            .for_each(|p| files.push(p));
    });

    write(
        &args.output,
        &to_string(&files).map_err(|e| anyhow!("Failed to stringify file list: {e}"))?,
    )
    .map_err(|e| anyhow!("Failed to write output to {}: {}", args.output.display(), e))?;

    Ok(())
}
