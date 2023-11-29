#!/bin/bash

if [ -z "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE must be set"
    exit 1
fi

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-x86.h" "${SCRIPT_DIR}/tsffs-gcc-x86.h"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

ninja

dd if=/dev/zero of=test.fs bs=1024 count=131072
mkfs.fat test.fs
mcopy -i test.fs test ::test
"${SIMICS_BASE}/linux64/bin/craff" -o test.fs.craff test.fs