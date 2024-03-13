// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! Tokenization of executables

use anyhow::Result;
use goblin::{pe::Coff, Object};
use libafl::prelude::{NaiveTokenizer, Tokenizer};
use std::{fs::read, path::Path};

// 3 character string minimum
const STRING_TOKEN_MIN_LEN: usize = 3;
// Counted in bytes, PE is 16-bit characters, so we need 4 bytes. We set this to 4, because
// PE strings can just be utf-8 as utf-16, so we don't want to double it.
const WCHAR_STRING_TOKEN_MIN_LEN: usize = 4;

pub fn tokenize_src_file<I, P>(source_files: I) -> Result<Vec<String>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut tokens = Vec::new();
    let tokenizer = NaiveTokenizer::default();

    source_files.into_iter().try_for_each(|f| {
        tokenizer
            .tokenize(&read(f.as_ref())?)
            .map(|t| tokens.extend(t))
    })?;

    Ok(tokens)
}

fn tokenize_strings(bytes: &[u8]) -> Result<Vec<Vec<u8>>> {
    const WCHAR_SIZE: usize = 2;
    let mut tokens = Vec::new();
    let mut remap_bytes = Vec::new();

    // Smush sequences of 0 to single 0s
    bytes.iter().for_each(|b| {
        if remap_bytes.last().is_some_and(|l| *l == 0) {
            // Nothing
        } else {
            remap_bytes.push(*b);
        }
    });

    remap_bytes
        .split(|b| *b == 0)
        .filter(|b| {
            // If we can interpret a nul-terminated slice as a utf-16 or utf-8 string, we take it
            // as a token
            (b.len() % 2 == 0
                && b.len() >= WCHAR_STRING_TOKEN_MIN_LEN
                && String::from_utf16(
                    &b.chunks_exact(WCHAR_SIZE)
                        // Big endian re-encode as &[u16]
                        .map(|c| (c[0] as u16) << 8 | c[1] as u16)
                        .collect::<Vec<_>>(),
                )
                .is_ok())
                || (b.len() >= STRING_TOKEN_MIN_LEN && String::from_utf8(b.to_vec()).is_ok())
        })
        .for_each(|b| tokens.push(b.to_vec()));

    Ok(tokens)
}

/// Naively tokenize an executable file by parsing its data sections. This very much assumes the
/// executable isn't behaving badly and that strings in it are actually in the data section.
///
/// For ELF executables, we take all non-executable and non-alloc sections.
///
/// For PE and COFF executables, we take the reserved sections .data and .rdata as noted in the
/// [docs](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#special-sections).
pub fn tokenize_executable_file<P>(executable: P) -> Result<Vec<Vec<u8>>>
where
    P: AsRef<Path>,
{
    let mut tokens = Vec::new();
    let contents = read(executable.as_ref())?;

    match Object::parse(&contents)? {
        Object::Elf(e) => {
            e.section_headers
                .iter()
                .filter(|sh| !sh.is_executable() && !sh.is_alloc())
                .filter_map(|sh| sh.file_range())
                .try_for_each(|range| {
                    tokenize_strings(&contents[range]).map(|t| tokens.extend(t))
                })?;
        }
        Object::PE(p) => {
            p.sections
                .iter()
                .filter(|s| s.name().is_ok_and(|n| n == ".rdata" || n == ".data"))
                .try_for_each(|s| {
                    tokenize_strings(
                        &contents[s.pointer_to_raw_data as usize
                            ..s.pointer_to_raw_data as usize + s.size_of_raw_data as usize],
                    )
                    .map(|t| tokens.extend(t))
                })?;
        }
        _ => {}
    }

    if let Ok(coff) = Coff::parse(&contents) {
        coff.sections
            .iter()
            .filter(|s| s.name().is_ok_and(|n| n == ".rdata" || n == ".data"))
            .try_for_each(|s| {
                tokenize_strings(
                    &contents[s.pointer_to_raw_data as usize
                        ..s.pointer_to_raw_data as usize + s.size_of_raw_data as usize],
                )
                .map(|t| tokens.extend(t))
            })?;
    }

    Ok(tokens)
}
