#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cp "${SCRIPT_DIR}/../../../harness/tsffs-gcc-x86_64.h" "${SCRIPT_DIR}/tsffs-gcc-x86_64.h"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

ninja