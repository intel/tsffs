// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

use crate::project::Project;

pub trait Setup {
    /// Set up some extra properties, files, or state around an existing SIMICS project
    /// specification
    fn setup(&self, project: &Project) -> Result<&Self>
    where
        Self: Sized;
}
