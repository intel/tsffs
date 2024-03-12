#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SIMICS_RS_SRC="$1"

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if [ ! -d "${SIMICS_RS_SRC}" ]; then
    echo "Error: argument should be the path to the simics-rs repository"
    exit 1
fi

if [ ! -f "${SIMICS_RS_SRC}/Cargo.toml" ]; then
    echo "Error: argument should be the path to the simics-rs repository"
    exit 1
fi

CURRENT_SIMICS_RS_SRC="${SCRIPT_DIR}/../simics-rs"

rm -rf "${CURRENT_SIMICS_RS_SRC}"
cp -a "${SIMICS_RS_SRC}" "${CURRENT_SIMICS_RS_SRC}"