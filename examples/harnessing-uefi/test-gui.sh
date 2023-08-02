#!/bin/bash

pushd src || exit 1
ninja
popd || exit 1

cargo run --manifest-path ../../Cargo.toml --release \
    --bin simics-fuzz --features=6.0.168 -- \
    --project ./project --input ./input --solutions ./solutions --corpus ./corpus \
    --log-level INFO --trace-mode once --executor-timeout 60 --timeout 3 --cores 1 \
    --package 2096:6.0.69 \
    --file "./src/target-harnessed.efi:%simics%/target.efi" \
    --file "./rsrc/fuzz.simics:%simics%/fuzz.simics" \
    --file "./rsrc/minimal_boot_disk.craff:%simics%/minimal_boot_disk.craff" \
    --command 'COMMAND:run-script "%simics%/fuzz.simics"' \
    --enable-simics-gui
