// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;
use test_packages::CARGO_MANIFEST_DIR;
use versions::Versioning;

#[test]
fn test_hello_world() -> Result<()> {
    TestEnvSpec::builder()
        .package_crates([PathBuf::from(CARGO_MANIFEST_DIR).join("../packages/hello-world")])
        .extra_packages([ProjectPackage::builder()
            .package_number(1000)
            .version(Versioning::new("latest").ok_or_else(|| anyhow!("Invalid version"))?)
            .build()])
        .name("hello-world")
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module HelloWorld
            @hw = SIM_create_object(SIM_get_class("HelloWorld"), "hw", [])
            @hw.message = "Hello, World!"
            @hw.iface.HelloWorldInterface.say()
            @hw.iface.HelloWorldInterface2.say2()
            @hw2 = SIM_create_object(SIM_get_class("HelloWorld2"), "hw2", [])
            @hw2.message = "Hello, World! (Again)"
            @hw2.iface.HelloWorld2Interface.say()
            @hw2.iface.HelloWorld2Interface2.say2()
        "#})?;

    Ok(())
}
