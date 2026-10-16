// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use indoc::indoc;
use ispm_wrapper::data::ProjectPackage;
use simics_test::TestEnvSpec;
use std::path::PathBuf;

#[test]
#[cfg_attr(miri, ignore)]
fn test_x86_64_edk2_exception_metadata_6_0_185() -> Result<()> {
    let output = TestEnvSpec::builder()
        .name("test_x86_64_edk2_exception_metadata_6_0_185")
        .package_crates([PathBuf::from(env!("CARGO_MANIFEST_DIR"))])
        .packages([
            ProjectPackage::builder()
                .package_number(1000)
                .version("6.0.185")
                .build(),
            ProjectPackage::builder()
                .package_number(1030)
                .version("6.0.185")
                .build(),
            ProjectPackage::builder()
                .package_number(2096)
                .version("6.0.73")
                .build(),
            ProjectPackage::builder()
                .package_number(8112)
                .version("6.0.21")
                .build(),
        ])
        .cargo_target_tmpdir(env!("CARGO_TARGET_TMPDIR"))
        .directories([PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rsrc")
            .join("x86_64-uefi-edk2")])
        // A single-byte testcase selecting the harness's AP-fault branch ('D'), armed as the
        // repro input so the very first (and only) execution deterministically exercises the
        // multi-processor fault, and the resulting stop is durable (not immediately cleared
        // by the next iteration's setup, unlike a stop during normal fuzzing).
        .files([("repro-input".to_string(), vec![b'D'])])
        .build()
        .to_env()?
        .test(indoc! {r#"
            load-module tsffs
            init-tsffs

            @tsffs.log_level = 2
            @tsffs.start_on_harness = True
            @tsffs.stop_on_harness = True
            @tsffs.timeout = 3.0
            @tsffs.exceptions = [14]
            @tsffs.generate_random_corpus = True
            @tsffs.iteration_limit = 1000
            @tsffs.iface.fuzz.repro("%simics%/repro-input")

            load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff" machine:hardware:processor:num_cores = 2

            script-branch {
                bp.time.wait-for seconds = 15
                qsp.serconsole.con.input "\n"
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "FS0:\n"
                bp.time.wait-for seconds = .5
                local $manager = (start-agent-manager)
                qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/test.efi") + "\n")
                bp.time.wait-for seconds = .5
                qsp.serconsole.con.input "test.efi\n"
            }

            script-branch {
                bp.time.wait-for seconds = 240
                quit 1
            }

            run

            @print(f"TSFFS_RESULT processor={tsffs.last_exception_processor_number} reason={tsffs.last_stop_reason}")
        "#})?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    println!("{output_str}");

    // Console-echoed lines may render line endings as the literal two-character sequence
    // `\r\n` rather than real control characters, so take only the leading digits rather
    // than trusting `trim`/`split_whitespace` to strip whatever follows a printed integer.
    fn leading_digits(s: &str) -> &str {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        &s[..end]
    }

    // The harness prints which AP (non-BSP processor) it dispatched the fault to.
    let ap_number: i32 = output_str
        .lines()
        .find_map(|line| {
            line.find("AP_FAULT_PROCESSOR ")
                .map(|idx| leading_digits(&line[idx + "AP_FAULT_PROCESSOR ".len()..]))
        })
        .filter(|s| !s.is_empty())
        .expect("Harness did not report AP_FAULT_PROCESSOR (insufficient processors, or MP Services unavailable)")
        .parse()
        .expect("AP_FAULT_PROCESSOR value was not an integer");

    // TSFFS reports which processor it attributed the exception-based solution to,
    // and the stop reason it recorded, through the new public attributes.
    let result_line = output_str
        .lines()
        .find(|line| line.contains("TSFFS_RESULT processor="))
        .expect("Simics script did not print TSFFS_RESULT");

    let reported_processor: i32 = result_line
        .split("processor=")
        .nth(1)
        .map(leading_digits)
        .filter(|s| !s.is_empty())
        .expect("Missing processor field in TSFFS_RESULT (last_exception_processor_number was likely Nil)")
        .parse()
        .expect("Reported processor was not an integer");

    assert_eq!(
        reported_processor, ap_number,
        "TSFFS reported the exception-based solution's processor as {reported_processor}, but \
         the harness actually targeted processor {ap_number}"
    );

    let reason = result_line
        .split("reason=")
        .nth(1)
        .expect("Missing reason field in TSFFS_RESULT");

    assert!(
        reason.contains("Exception") && reason.contains("14"),
        "last_stop_reason did not describe an exception 14 solution: {reason}"
    );

    Ok(())
}
