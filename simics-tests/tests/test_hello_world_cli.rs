use std::fs::{read_dir, remove_dir_all, write};

use anyhow::Result;
use clap::Parser;
use simics_fuzz::{args::Args, fuzzer::SimicsFuzzer};

const CARGO_MANIFEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../");
const ITERATIONS: usize = 3;

#[test]
fn test_hello_world_cli() -> Result<()> {
    use tempdir::TempDir;

    let tmp_input_dir = TempDir::new("test_hello_world_cli_input")?.into_path();
    let tmp_corpus_dir = TempDir::new("test_hello_world_cli_corpus")?.into_path();

    // For this test, we set up a corpus
    write(tmp_input_dir.join("1"), "racecar".as_bytes())?;

    let tmp_solution_dir = TempDir::new("test_hello_world_cli_solution")?.into_path();

    let args = &[
        "simics-fuzz",
        "-i",
        &tmp_input_dir.to_string_lossy(),
        "-c",
        &tmp_corpus_dir.to_string_lossy(),
        "-s",
        &tmp_solution_dir.to_string_lossy(),
        "-l",
        "INFO",
        "-C",
        "1",
        "--iterations",
        &format!("{}", ITERATIONS),
        "--no-keep-temp-projects",
        "--package",
        "2096:6.0.66",
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/HelloWorld.efi:%simics%/targets/hello-world/HelloWorld.efi", CARGO_MANIFEST_DIR),
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/app.py:%simics%/scripts/app.py", CARGO_MANIFEST_DIR),
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/app.yml:%simics%/scripts/app.yml", CARGO_MANIFEST_DIR),
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/minimal_boot_disk.craff:%simics%/targets/hello-world/minimal_boot_disk.craff", CARGO_MANIFEST_DIR),
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/run_uefi_app.nsh:%simics%/targets/hello-world/run_uefi_app.nsh", CARGO_MANIFEST_DIR),
        "--file",
        &format!("{}/targets/hello-world/src/bin/resource/run-uefi-app.simics:%simics%/targets/hello-world/run-uefi-app.simics", CARGO_MANIFEST_DIR),
        "--command",
        "CONFIG:%simics%/scripts/app.yml",
    ];

    println!("{}", args.join(" "));

    let args = Args::parse_from(args);

    println!("{:?}", args);

    SimicsFuzzer::cli_main(args)?;

    // Check to make sure we have some solution
    assert!(
        read_dir(&tmp_solution_dir)?.count() > 0,
        "No solutions found in {} iterations",
        ITERATIONS
    );

    assert!(
        read_dir(&tmp_corpus_dir)?.count() > 0,
        "No corpus in {} iterations",
        ITERATIONS
    );
    remove_dir_all(&tmp_input_dir)?;
    remove_dir_all(&tmp_corpus_dir)?;
    remove_dir_all(&tmp_solution_dir)?;
    Ok(())
}
