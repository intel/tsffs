#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if ! command -v cargo-llvm-cov &>/dev/null; then
    echo "cargo-llvm-cov must be installed! Run 'cargo install cargo-llvm-cov'"
    exit 1
fi

pushd "${SCRIPT_DIR}" || exit 1

cargo llvm-cov
