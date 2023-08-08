// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use adv::{Config, KittyConfigBuilder, KittyProcess, Secrets};
use anyhow::{anyhow, Result};
use clap::Parser;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::{
    fs::read_to_string,
    io::{stdin, stdout, Write},
    path::PathBuf,
};
use termion::input::TermRead;
use toml::from_str;
use tracing::metadata::LevelFilter;
use tracing_subscriber::{fmt, prelude::*, registry, Layer};

#[derive(Parser)]
struct Args {
    config: PathBuf,
    #[arg(short = 'L', long, default_value_t = LevelFilter::INFO)]
    log_level: LevelFilter,
    #[arg(short, long)]
    purge_cache: bool,
}

fn read_secret(name: &str) -> Result<String> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(format!("{}: ", name).as_bytes())?;
    stdout.flush()?;

    let pass = stdin
        .read_passwd(&mut stdout)?
        .ok_or_else(|| anyhow!("Error reading password"))?;

    stdout.write_all(b"\n")?;

    Ok(pass)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut config: Config = from_str(&read_to_string(args.config)?)?;

    let reg = registry().with({
        fmt::layer()
            .pretty()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_writer(stdout)
            .with_filter(args.log_level)
    });
    reg.try_init()?;

    let mut secrets = Secrets::new(config.secrets_file)?;
    let sock = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>();

    if let Some(secret_keys) = &config.secrets {
        for secret in secret_keys {
            if secrets.get(secret).is_err() {
                secrets.add(
                    secret.trim().to_string(),
                    read_secret(secret)?.trim().to_string(),
                );
            }
        }
    }

    if let Some(global_wpm) = config.wpm {
        for entry in config.command.iter_mut() {
            if entry.wpm.is_none() {
                entry.wpm = Some(global_wpm);
            }
        }
    }

    let mut kitty = KittyProcess::try_new(
        KittyConfigBuilder::default()
            .listen_on(format!("unix:@{}", sock))
            .title("Demo")
            .overrides(config.kitty.unwrap_or_default().to_overrides())
            .build()?,
        &secrets,
    )?;

    for command in config.command {
        command.exec(&kitty, &secrets)?;
    }

    if config.keep_open.is_some_and(|k| k) {
        print!("Press Enter to finish: ");
        stdout().flush()?;
        stdin()
            .lines()
            .take(1)
            .next()
            .ok_or_else(|| anyhow!("Failed to get a line"))??;
    }

    kitty.kill()?;
    Ok(())
}
