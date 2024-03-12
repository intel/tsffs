#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

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

if ! command -v markdownlint &>/dev/null; then
    echo "markdownlint must be installed! Install with 'npm i -g markdownlint-cli'"
    exit 1
fi

echo "================="
echo "Formatting Rust"
echo "================="

cargo fmt --all

echo "================="
echo "Formatting Markdown"
echo "================="

fd '.*\.md$' -x markdownlint -f -c "${SCRIPT_DIR}/../.github/linters/.markdown-lint.yml" {}
