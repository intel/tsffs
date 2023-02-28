use anyhow::{bail, ensure, Result};
use confuse_simics_manifest::{simics_latest, PackageNumber};
use confuse_simics_modsign::generate_signature_header;
use dockerfile_rs::{Copy, DockerFile, From, TagOrDigest, RUN, WORKDIR};
use dotenvy_macro::dotenv;
use std::{
    env::var,
    fs::{read_dir, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
};

const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
const EDK2_REPO_URL: &str = "https://github.com/tianocore/edk2.git";
const EDK2_REPO_HASH: &str = "02fcfdce1e5ce86f1951191883e7e30de5aa08be";
const EDK2_FEDORA35_REPO_URL: &str = "ghcr.io/tianocore/containers/fedora-35-build";
const EDK2_FEDORA35_BUILD_TAG: &str = "5b8a008";

/// Return the SIMICS_HOME directory as a PathBuf
fn simics_home() -> Result<PathBuf> {
    let simics_home = PathBuf::from(SIMICS_HOME);
    match simics_home.exists() {
        true => Ok(simics_home),
        false => {
            bail!(
                "SIMICS_HOME is defined, but {} does not exist.",
                SIMICS_HOME
            )
        }
    }
}

/// Return the OUT_DIR build directory as a PathBuf
fn out_dir() -> Result<PathBuf> {
    match var("OUT_DIR") {
        Ok(out_dir) => Ok(PathBuf::from(out_dir)),
        Err(e) => Err(e.into()),
    }
}

fn write_simics_constants() -> Result<()> {
    let simics_home = simics_home()?;
    let crate_name = var("CARGO_PKG_NAME")?;
    let simics_module_header_path = PathBuf::from(var("OUT_DIR")?).join("simics_module_header.rs");

    let header_contents = generate_signature_header(crate_name, simics_home)?;

    let mut simics_module_header = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&simics_module_header_path)?;

    write!(&mut simics_module_header, "{}", header_contents)?;

    Ok(())
}

fn link_simics() -> Result<()> {
    let simics_bin_dir = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("bin");

    let simics_sys_lib_dir = simics_home()?
        .join(format!(
            "simics-{}",
            simics_latest(simics_home()?)?.packages[&PackageNumber::Base].version
        ))
        .join("linux64")
        .join("sys")
        .join("lib");

    println!(
        "cargo:rustc-link-search=native={}",
        simics_bin_dir.display()
    );

    println!(
        "cargo:rustc-link-search=native={}",
        simics_sys_lib_dir.display()
    );

    println!("cargo:rustc-link-lib=simics-common");
    println!("cargo:rustc-link-lib=vtutils");
    println!("cargo:rustc-link-lib=package-paths");
    // TODO: Get this full path from the simics lib
    println!("cargo:rustc-link-lib=dylib:+verbatim=libpython3.9.so.1.0");

    // NOTE: This only works for `cargo run` and `cargo test` and won't work for just running
    // the output binary
    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={}",
        &format!(
            "{};{}",
            simics_bin_dir.to_string_lossy(),
            simics_sys_lib_dir.to_string_lossy(),
        )
    );
    Ok(())
}

fn build_efi_module() -> Result<()> {
    let manifest_dir = PathBuf::from(var("CARGO_MANIFEST_DIR")?);
    let module_src_path = manifest_dir.join("module");

    read_dir(&module_src_path)?
        .filter_map(|p| p.ok())
        .for_each(|de| {
            let p = de.path();
            if p.is_file() {
                if let Ok(rp) = &p.strip_prefix(&manifest_dir) {
                    println!("cargo:rerun-if-changed={}", rp.to_string_lossy());
                }
            }
        });

    ensure!(
        module_src_path.is_dir(),
        "Module source directory does not exist."
    );
    let dockerfile_contents = DockerFile::from(From {
        image: EDK2_FEDORA35_REPO_URL.to_string(),
        tag_or_digest: Some(TagOrDigest::Tag(EDK2_FEDORA35_BUILD_TAG.to_string())),
        name: None,
    })
    .work_dir(WORKDIR!("/"))
    .run(RUN!["git", "clone", EDK2_REPO_URL, "/edk2"])
    .work_dir(WORKDIR!("/edk2"))
    .run(RUN!["git", "-C", "/edk2", "checkout", EDK2_REPO_HASH])
    // TODO: Can we use a relative path here, ensure it exists, etc?
    .run(RUN![
        "python3",
        "-m",
        "pip",
        "install",
        "-r",
        "/edk2/pip-requirements.txt"
    ])
    .run(RUN![
        "stuart_setup",
        "-c",
        "/edk2/.pytool/CISettings.py",
        "TOOL_CHAIN_TAG=GCC5"
    ])
    .run(RUN![
        "stuart_update",
        "-c",
        "/edk2/.pytool/CISettings.py",
        "TOOL_CHAIN_TAG=GCC5"
    ])
    .copy(Copy {
        src: module_src_path
            .strip_prefix(&manifest_dir)?
            .to_string_lossy()
            .to_string(),
        dst: "/edk2/HelloWorld/".to_string(),
        chown: None,
        from: None,
    })
    .run(RUN![
        "stuart_setup",
        "-c",
        "/edk2/HelloWorld/PlatformBuild.py",
        "TOOL_CHAIN_TAG=GCC5"
    ])
    .run(RUN![
        "stuart_update",
        "-c",
        "/edk2/HelloWorld/PlatformBuild.py",
        "TOOL_CHAIN_TAG=GCC5"
    ])
    .run(RUN![
        "python3",
        "/edk2/BaseTools/Edk2ToolsBuild.py",
        "-t",
        "GCC5"
    ])
    .run(RUN![
        "bash",
        "-c",
        "source /edk2/edksetup.sh \
            && stuart_build -c /edk2/HelloWorld/PlatformBuild.py \
               TOOL_CHAIN_TAG=GCC5 EDK_TOOLS_PATH=/edk2/BaseTools/ \
            || ( cat /edk2/HelloWorld/Build/BUILDLOG.txt && exit 1 )"
    ]);

    // TODO: We should probably use a real docker API but bollard is async and nothing else is
    // updated
    let dockerfile_path = out_dir()?.join("Dockerfile");
    let hello_world_efi_out_path = out_dir()?.join("HelloWorld.efi");
    let docker_build_ctx_path = manifest_dir;
    let mut dockerfile = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&dockerfile_path)?;

    write!(&mut dockerfile, "{}", dockerfile_contents)?;

    let build_status = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg("edk2-build-hello-world")
        .arg("-f")
        .arg(&dockerfile_path)
        .arg(&docker_build_ctx_path)
        .status()?;

    ensure!(build_status.success(), "Build error: {}", build_status);

    let create_status = Command::new("docker")
        .arg("create")
        .arg("--name")
        .arg("edk2-build-hello-world-tmp")
        .arg("edk2-build-hello-world")
        .status()?;

    ensure!(create_status.success(), "create error: {}", create_status);

    let cp_status = Command::new("docker")
        .arg("cp")
        .arg("edk2-build-hello-world-tmp:/edk2/HelloWorld/Build/HelloWorld/DEBUG_GCC5/X64/HelloWorld.efi")
        .arg(&hello_world_efi_out_path)
        // Ignore errors here, we will need to rm for cleanup regardless
        .status()?;

    ensure!(cp_status.success(), "cp error: {}", cp_status);

    let rm_status = Command::new("docker")
        .arg("rm")
        .arg("-f")
        .arg("edk2-build-hello-world-tmp")
        .status()?;

    ensure!(rm_status.success(), "rm error: {}", rm_status);

    Ok(())
}

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    write_simics_constants()?;
    link_simics()?;
    build_efi_module()?;
    Ok(())
}
