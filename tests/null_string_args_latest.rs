// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Regression test for <https://github.com/intel/tsffs/issues/123>: passing `None` (NULL)
//! as a string interface argument must fail with an error, not dereference NULL.

use command_ext::CommandExtError;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_null_string_args_latest() -> anyhow::Result<()> {
    let result = TestEnvSpec::builder()
        .name("test_null_string_args_latest")
        .package_crates([PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .packages([ProjectPackage::builder()
            .package_number(1000)
            .version("latest")
            .build()])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .build()
        .to_env()?
        .test_python(indoc! {r#"
            import simics

            simics.SIM_load_module("tsffs")
            tsffs = simics.SIM_create_object(simics.SIM_get_class("tsffs"), "tsffs", [])
            tsffs.iface.fuzz.solution(1, None)
        "#});

    // Interface errors abort Simics via the generated `.expect()`; the panic must be ours,
    // not a segfault from dereferencing NULL.
    let Err(CommandExtError::Check { stdout, stderr, .. }) = result else {
        anyhow::bail!("expected Simics to fail, got {result:?}");
    };
    anyhow::ensure!(
        stderr.contains("must not be NULL") || stdout.contains("must not be NULL"),
        "missing NULL error\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    Ok(())
}
