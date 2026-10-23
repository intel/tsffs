// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Checks that `harness/tsffs-gcc-arm32.h` places the magic index and arguments in the
//! registers `src/arch/arm.rs` reads them from (index: r10, args: r9, r8, r7). No public
//! Simics package models an ARM32 CPU, so this compiles the harness with clang and
//! constant-propagates each function up to its magic `orr rN, rN, rN` instruction.

use anyhow::{anyhow, bail, ensure, Result};
use goblin::elf::{sym::STT_FUNC, Elf};
use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};
use yaxpeax_arch::{Decoder, U8Reader};
use yaxpeax_arm::armv7::{InstDecoder, Opcode, Operand, RegShiftStyle};

const INDEX_REGISTER: usize = 10;
const ARGUMENT_REGISTERS: [usize; 3] = [9, 8, 7];

const SOURCE: &str = r#"
#include "tsffs-gcc-arm32.h"
#define BUF ((void *)0x1000)
#define SIZE_PTR ((void *)0x2000)
#define MAX_SIZE 0x3000
void start(void) { HARNESS_START(BUF, SIZE_PTR); }
void start_index(void) { HARNESS_START_INDEX(7, BUF, SIZE_PTR); }
void start_max(void) { HARNESS_START_WITH_MAXIMUM_SIZE(BUF, MAX_SIZE); }
void start_max_index(void) { HARNESS_START_WITH_MAXIMUM_SIZE_INDEX(7, BUF, MAX_SIZE); }
void start_max_ptr(void) { HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR(BUF, SIZE_PTR, MAX_SIZE); }
void start_max_ptr_index(void) { HARNESS_START_WITH_MAXIMUM_SIZE_AND_PTR_INDEX(7, BUF, SIZE_PTR, MAX_SIZE); }
void stop(void) { HARNESS_STOP(); }
void stop_index(void) { HARNESS_STOP_INDEX(7); }
void assert_(void) { HARNESS_ASSERT(); }
void assert_index(void) { HARNESS_ASSERT_INDEX(7); }
"#;

/// Register values at the magic instruction: (magic number, index, arguments).
type Magic = (u8, Option<u32>, [Option<u32>; 3]);

/// Walk straight-line ARM code, tracking registers loaded with constants, and snapshot them at
/// the first magic instruction. Any other instruction writing a register makes it unknown, so
/// unsupported code can only fail the test, never pass it.
fn magic_registers(code: &[u8]) -> Result<Magic> {
    let decoder = InstDecoder::default();
    let mut reader = U8Reader::new(code);
    let mut regs = [None::<u32>; 16];
    while let Ok(insn) = decoder.decode(&mut reader) {
        let dest = match insn.operands[0] {
            Operand::Reg(r) | Operand::RegWBack(r, _) => r.number() as usize,
            _ => continue,
        };
        match (insn.opcode, insn.operands[1], insn.operands[2]) {
            (Opcode::ORR, Operand::Reg(a), Operand::Reg(b))
                if a.number() as usize == dest && b.number() as usize == dest =>
            {
                let args = ARGUMENT_REGISTERS.map(|r| regs[r]);
                return Ok((dest as u8, regs[INDEX_REGISTER], args));
            }
            (Opcode::MOV, Operand::Imm32(v), Operand::Nothing) => regs[dest] = Some(v),
            (Opcode::MOV, Operand::Imm12(v), Operand::Nothing) => regs[dest] = Some(v.into()),
            (Opcode::MOV, Operand::Reg(src), Operand::Nothing) => {
                regs[dest] = regs[src.number() as usize]
            }
            (Opcode::MOV, Operand::RegShift(rs), Operand::Nothing) => {
                regs[dest] = match rs.into_shift() {
                    RegShiftStyle::RegImm(s) if s.imm() == 0 => regs[s.shiftee().number() as usize],
                    _ => None,
                }
            }
            _ => regs[dest] = None,
        }
    }
    bail!("no magic instruction found")
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_arm32_harness_abi_latest() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("arm32_harness_abi");
    fs::create_dir_all(&out_dir)?;
    let (src, obj) = (out_dir.join("abi.c"), out_dir.join("abi.o"));
    fs::write(&src, SOURCE)?;

    let status = Command::new("clang")
        .args([
            "-target",
            "arm-unknown-linux-gnueabi",
            "-marm",
            "-mfloat-abi=soft",
            "-O2",
        ])
        .arg("-I")
        .arg(manifest_dir.join("harness"))
        .arg("-c")
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .status()?;
    ensure!(
        status.success(),
        "clang failed to compile the arm32 harness"
    );

    let bytes = fs::read(&obj)?;
    let elf = Elf::parse(&bytes)?;
    let mut functions = BTreeMap::new();
    for sym in elf.syms.iter().filter(|s| s.st_type() == STT_FUNC) {
        let name = elf
            .strtab
            .get_at(sym.st_name)
            .ok_or_else(|| anyhow!("bad symbol name"))?;
        let section = &elf.section_headers[sym.st_shndx];
        let start = (section.sh_offset + sym.st_value) as usize;
        functions.insert(name, &bytes[start..start + sym.st_size as usize]);
    }

    let (buf, size_ptr, max_size) = (Some(0x1000), Some(0x2000), Some(0x3000));
    let expected: [(&str, Magic); 10] = [
        ("start", (1, Some(0), [buf, size_ptr, None])),
        ("start_index", (1, Some(7), [buf, size_ptr, None])),
        ("start_max", (2, Some(0), [buf, max_size, None])),
        ("start_max_index", (2, Some(7), [buf, max_size, None])),
        ("start_max_ptr", (3, Some(0), [buf, size_ptr, max_size])),
        (
            "start_max_ptr_index",
            (3, Some(7), [buf, size_ptr, max_size]),
        ),
        ("stop", (4, Some(0), [None; 3])),
        ("stop_index", (4, Some(7), [None; 3])),
        ("assert_", (5, Some(0), [None; 3])),
        ("assert_index", (5, Some(7), [None; 3])),
    ];
    for (name, (magic, index, args)) in expected {
        let code = functions
            .get(name)
            .ok_or_else(|| anyhow!("missing {name}"))?;
        let (got_magic, got_index, got_args) = magic_registers(code)?;
        assert_eq!(got_magic, magic, "{name}: magic number");
        assert_eq!(got_index, index, "{name}: index register r10");
        // Only arguments the magic number consumes are part of the contract.
        for (i, want) in args.iter().enumerate().filter(|(_, a)| a.is_some()) {
            assert_eq!(
                got_args[i], *want,
                "{name}: argument register r{}",
                ARGUMENT_REGISTERS[i]
            );
        }
    }
    Ok(())
}
