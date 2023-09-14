#!/bin/bash

# Copyright (C) 2023 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

pushd src || exit 1
ninja
popd || exit 1

cargo run --manifest-path ../../Cargo.toml --release \
    --features=6.0.169 -- \
    --project ./project --input ./input --solutions ./solutions --corpus ./corpus \
    --log-level INFO --trace-mode once --executor-timeout 60 --timeout 3 --cores 1 \
    --package 2096:6.0.70 \
    --file "./src/target-harnessed-include.efi:%simics%/target.efi" \
    --file "./rsrc/fuzz.simics:%simics%/fuzz.simics" \
    --file "./rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff" \
    --command 'COMMAND:run-script "%simics%/fuzz.simics"' \
    --enable-simics-gui
