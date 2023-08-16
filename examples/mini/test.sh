#!/bin/bash

# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

pushd "${SCRIPT_DIR}" || exit 1

pushd "${SCRIPT_DIR}/src/" || exit 1

ninja

popd || exit 1

cargo run --release --bin simics-fuzz --features=6.0.170 -- \
    -p test-project -i corpus -c corpus -s solution -l INFO -L log.txt -C 1 \
    --package 2096:6.0.69 \
    --file "${SCRIPT_DIR}/rsrc/mini.efi:%simics%/mini.efi" \
    --file "${SCRIPT_DIR}/rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff" \
    --file "${SCRIPT_DIR}/rsrc/fuzz.simics:%simics%/fuzz.simics" \
    --command 'COMMAND:run-script "%simics%/fuzz.simics"'
