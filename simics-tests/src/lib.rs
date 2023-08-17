// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#![deny(clippy::unwrap_used)]
#![forbid(unsafe_code)]

use anyhow::Result;
use simics::{
    api::{GuiMode, InitArg, InitArgs},
    simics::Simics,
};

pub fn test_init() -> Result<()> {
    let args = InitArgs::default()
        .arg(InitArg::batch_mode()?)
        .arg(InitArg::gui_mode(GuiMode::None.to_string())?)
        .arg(InitArg::quiet()?)
        .arg(InitArg::no_windows()?)
        .arg(InitArg::no_settings()?);

    Simics::try_init(args)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, str::FromStr};

    use super::test_init;
    use anyhow::Result;
    use simics::project::SimicsPath;

    #[test]
    #[cfg_attr(miri, ignore)]
    pub fn init() -> Result<()> {
        test_init()
    }

    #[test]
    pub fn test_simics_path_traversal_reject() -> Result<()> {
        let base = PathBuf::from("/tmp/");
        let out_of_base_path = SimicsPath::from_str("%simics%/../bad.txt")?;
        assert!(
            out_of_base_path.canonicalize(base).is_err(),
            "Canonicalization should not be allowed outside of simics base"
        );
        Ok(())
    }

    #[test]
    pub fn test_simics_path_canonicalize() -> Result<()> {
        let base = PathBuf::from("/tmp/");
        let in_base_path = SimicsPath::from_str("%simics%/bad.txt")?;

        let canonicalized = in_base_path.canonicalize(base)?;
        assert!(
            canonicalized.is_absolute(),
            "Canonicalization should not be allowed outside of simics base"
        );
        Ok(())
    }
}
