#!/bin/bash

# Clang-Format

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if ! command -v cargo &>/dev/null; then
    echo "cargo must be installed! Install from https://rustup.rs"
    exit 1
fi

if ! command -v fd &>/dev/null; then
    echo "fd must be installed! Install with 'cargo install fd-find'"
    exit 1
fi

if ! command -v clang-format &>/dev/null; then
    echo "clang-format must be installed! Install with your system package manager."
    exit 1
fi

if ! command -v black &>/dev/null; then
    echo "black must be installed! Install with 'python3 -m pip install black'"
    exit 1
fi

if ! command -v isort &>/dev/null; then
    echo "black must be installed! Install with 'python3 -m pip install black'"
    exit 1
fi

echo "Formatting C/C++"

fd '.*(\.h|\.c|\.cc|\.hh)$' -x clang-format -i {}

echo "Formatting Rust"

cargo fmt --all

fd '.*\.py$' -x black --config "${SCRIPT_DIR}/../.github/linters/.python-black" {}
fd '.*\.py$' -x isort {}
