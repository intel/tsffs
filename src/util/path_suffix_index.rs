// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

//! A small, standalone "path-suffix index" primitive.
//!
//! Extracted out of `crate::source_cov::SourceCache`, which originally built and
//! queried exactly this structure (its `prefix_lookup` field and
//! `lookup_file_name_components` method) inline, coupled to `SourceCache::new`'s
//! per-file MD5/SHA1/SHA256 content hashing. The UEFI module discovery milestone
//! (UCOV-M2) needs the same "resolve an incoming path by longest matching path-
//! component suffix against a local directory tree" behavior over `.debug`/`.efi`
//! binaries, which don't need (and shouldn't pay the cost of) content hashing.
//! Factoring this out lets both `SourceCache` and the UEFI resolver
//! (`crate::uefi::resolve_debug_info`) share the same logic without either paying
//! for the other's unrelated work.
//!
//! # How it works
//!
//! Every local file path under a root directory is broken into its "normal" path
//! components (i.e. excluding roots, drive prefixes, `.`/`..`). For a path with
//! components `[a, b, c, d]`, every suffix is indexed: `[a, b, c, d]`, `[b, c, d]`,
//! `[c, d]`, `[d]`. Looking up an incoming path tries the same suffixes, longest
//! first, against that index -- so a fully-qualified incoming path that shares a
//! long tail with exactly one local file (e.g. `.../PkgA/.../DEBUG/Foo.efi` vs
//! `.../PkgB/.../DEBUG/Foo.efi`, when only one of `PkgA`/`PkgB` also exists as a
//! local directory name) disambiguates correctly, while a bare filename still
//! falls back to matching on just `[d]` if nothing longer matches.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Result;
use typed_path::{TypedComponent, TypedPath, UnixComponent, WindowsComponent};
use walkdir::WalkDir;

/// An index of local file paths, keyed by every suffix of their path components,
/// supporting "longest matching suffix" lookups.
///
/// Each suffix key maps to *every* local path sharing that suffix (not just the
/// most-recently-inserted one), so callers can tell a genuinely unique match from
/// an ambiguous one -- see [`PathSuffixIndex::lookup_components_unambiguous`].
#[derive(Debug, Clone, Default)]
pub struct PathSuffixIndex {
    suffixes: HashMap<Vec<String>, Vec<PathBuf>>,
}

impl PathSuffixIndex {
    /// Construct an empty index. Use [`PathSuffixIndex::insert`] to populate it
    /// incrementally (e.g. from a caller's own directory walk that is already doing
    /// other per-file work, like `SourceCache::new`'s content hashing), or
    /// [`PathSuffixIndex::build_from_dir`] to walk a directory and populate it in
    /// one call with no other per-file work.
    pub fn new() -> Self {
        Self::default()
    }

    /// Walk `root` and index every file found under it. This does not read file
    /// contents at all (unlike `SourceCache::new`), so it's cheap to use over
    /// directories of large binaries (`.debug`/`.efi`) that don't need hashing.
    pub fn build_from_dir<P>(root: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut index = Self::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
        {
            index.insert(entry.path());
        }

        Ok(index)
    }

    /// Index a single local file path under every suffix of its normal path
    /// components.
    pub fn insert(&mut self, path: &Path) {
        let mut components = Self::normal_components_of_path(path);

        // Insert the full component list, then progressively drop the first
        // (leftmost/outermost) component and insert the remainder, down to just the
        // file name. This means every suffix of the path is a key, and the longest
        // key that matches an incoming path is the most specific match.
        while !components.is_empty() {
            self.suffixes
                .entry(components.clone())
                .or_default()
                .push(path.to_path_buf());
            components.remove(0);
        }
    }

    /// Find the candidates (every local path sharing that exact suffix) for the
    /// longest matching suffix of `components`, trying progressively-shortened
    /// suffixes (longest first, i.e. dropping leading components one at a time)
    /// until some suffix length has at least one candidate.
    fn lookup_candidates(&self, components: &[String]) -> Option<&[PathBuf]> {
        let mut components = components.to_vec();

        while !components.is_empty() {
            if let Some(candidates) = self.suffixes.get(&components) {
                return Some(candidates);
            }

            components.remove(0);
        }

        None
    }

    /// Best-effort lookup: returns *some* local path matching the longest matching
    /// suffix, without regard for whether other local paths also share that exact
    /// suffix (in which case one is picked arbitrarily, but deterministically for a
    /// given index). This matches the pre-refactor behavior of `SourceCache`'s own
    /// inline lookup, which used a plain last-insert-wins
    /// `HashMap<Vec<String>, PathBuf>` and never detected ambiguity; source file
    /// basename collisions across a source tree are assumed rare enough not to need
    /// explicit disambiguation for that caller. Use
    /// [`PathSuffixIndex::lookup_components_unambiguous`] instead when ambiguity
    /// must be detected rather than silently resolved.
    pub fn lookup_components(&self, components: &[String]) -> Option<&Path> {
        self.lookup_candidates(components)
            .and_then(|candidates| candidates.first())
            .map(|p| p.as_path())
    }

    /// Convenience wrapper over [`PathSuffixIndex::lookup_components`] that accepts
    /// an incoming path as a plain string, which may use either Unix or Windows
    /// path separators (e.g. an embedded build-machine path recorded on a different
    /// OS than this one is running on) -- exactly the case `SourceCache` originally
    /// handled via `typed_path`.
    pub fn lookup_str(&self, path_str: &str) -> Option<&Path> {
        self.lookup_components(&Self::normal_components_of_str(path_str))
    }

    /// Strict lookup: like [`PathSuffixIndex::lookup_components`], but a match is
    /// only returned if the longest matching suffix is *unambiguous* (exactly one
    /// local path shares it). An ambiguous suffix stops the search immediately
    /// (returns `None`) rather than falling through to check a shorter suffix --
    /// a shorter suffix's candidate set is always a superset of a longer suffix's
    /// (every path sharing the longer suffix also shares every shorter suffix of
    /// it), so a shorter suffix can never be less ambiguous.
    ///
    /// This is what the UEFI module discovery resolver
    /// (`crate::uefi::resolve_one`) uses: it needs to know when suffix-matching
    /// genuinely couldn't disambiguate two same-named modules, so it can fall back
    /// to a different strategy (bare-stem search) instead of silently guessing.
    pub fn lookup_components_unambiguous(&self, components: &[String]) -> Option<&Path> {
        match self.lookup_candidates(components) {
            Some([single]) => Some(single.as_path()),
            _ => None,
        }
    }

    /// String-accepting convenience wrapper over
    /// [`PathSuffixIndex::lookup_components_unambiguous`], analogous to
    /// [`PathSuffixIndex::lookup_str`].
    pub fn lookup_str_unambiguous(&self, path_str: &str) -> Option<&Path> {
        self.lookup_components_unambiguous(&Self::normal_components_of_str(path_str))
    }

    /// Decompose a local, real `Path` into its normal ("real name") components as
    /// strings, dropping roots/prefixes/`.`/`..`.
    fn normal_components_of_path(path: &Path) -> Vec<String> {
        path.components()
            .filter_map(|c| {
                if let std::path::Component::Normal(c) = c {
                    Some(c.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Decompose a path-like string, which may be Unix- or Windows-style, into its
    /// normal components as strings. Uses `typed_path` since incoming embedded
    /// paths may have been recorded on a different OS than this one, so plain
    /// `std::path::Path` component parsing (which assumes the host OS's separator
    /// conventions) is not reliable for them.
    fn normal_components_of_str(path_str: &str) -> Vec<String> {
        TypedPath::derive(&path_str.to_string())
            .components()
            .filter_map(|c| match c {
                TypedComponent::Unix(u) => {
                    if let UnixComponent::Normal(c) = u {
                        String::from_utf8(c.to_vec()).ok()
                    } else {
                        None
                    }
                }
                TypedComponent::Windows(w) => {
                    if let WindowsComponent::Normal(c) = w {
                        String::from_utf8(c.to_vec()).ok()
                    } else {
                        None
                    }
                }
            })
            .collect()
    }
}
