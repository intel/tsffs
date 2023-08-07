// Copyright (C) 2023 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

void module_init_local(void);

/// Called automatically by SIMICS
///
/// We use this as a stub to call the real initialize function in our linked
/// library
void init_local() { module_init_local(); }