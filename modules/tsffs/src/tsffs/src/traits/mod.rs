// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

pub trait Component {
    /// Called after the initial snapshot is taken
    fn on_start(&mut self) -> Result<()>;
}
