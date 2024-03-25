#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
CRAFF="${SCRIPT_DIR}/../../../bin/craff"
CRAFF_FS="${SCRIPT_DIR}/../../../bin/craff-fs"

if [ -n "${SIMICS_BASE}" ]; then
    mkdir -p "${SCRIPT_DIR}/../../../bin"
    cp "${SIMICS_BASE}/linux64/bin/craff"  "${CRAFF}"
    cp "${SIMICS_BASE}/linux64/bin/craff-fs"  "${CRAFF_FS}"
fi


cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/tsffs.h"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

ninja

dd if=/dev/zero of=test.fs bs=1024 count=131072
mkfs.fat test.fs
mcopy -i test.fs test ::test
"${CRAFF}" -o test.fs.craff test.fs