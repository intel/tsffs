#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

pushd "${SCRIPT_DIR}" || exit 1

cargo run --manifest-path ../adv/Cargo.toml --bin adv fuzzing.toml
