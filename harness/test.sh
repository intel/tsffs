#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

# Compile test.c for each of x86, x86_64, riscv32, riscv64 architecture

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

set -e

"${SCRIPT_DIR}/build.sh"

rm -f "${SCRIPT_DIR}/test_x86_64-clang.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86-clang.o" || exit 0
rm -f "${SCRIPT_DIR}/test_riscv32-clang.o" || exit 0
rm -f "${SCRIPT_DIR}/test_riscv64-clang.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86_64-gcc.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86-gcc.o" || exit 0
rm -rf "${SCRIPT_DIR}/test_aarch64-clang.o" || exit 0
rm -rf "${SCRIPT_DIR}/test_arm32-clang.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86_64-clang-single-file.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86-clang-single-file.o" || exit 0
rm -f "${SCRIPT_DIR}/test_riscv32-clang-single-file.o" || exit 0
rm -f "${SCRIPT_DIR}/test_riscv64-clang-single-file.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86_64-gcc-single-file.o" || exit 0
rm -f "${SCRIPT_DIR}/test_x86-gcc-single-file.o" || exit 0
rm -rf "${SCRIPT_DIR}test_aarch64-clang-single-file.o" || exit 0
rm -rf "${SCRIPT_DIR}test_arm32-clang-single-file.o" || exit 0

echo "Testing x86_64 (single file)..."
clang -target x86_64-unknown-linux-gnu -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86_64-clang-single-file.o"
echo "Testing i386 (single file)..."
clang -target i386-unknown-linux-gnu -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86-clang-single-file.o"
echo "Testing riscv32 (single file)..."
clang -target riscv32-unknown-linux-gnu -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_riscv32-clang-single-file.o"
echo "Testing riscv64 (single file)..."
clang -target riscv64-unknown-linux-gnu -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_riscv64-clang-single-file.o"
echo "Testing aarch64 (single file)..."
clang -target aarch64-unknown-linux-gnu -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_aarch64-clang-single-file.o"
echo "Testing arm (single file)..."
clang -target arm-unknown-linux-gnu -mfloat-abi=soft -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_arm32-clang-single-file.o"
echo "Testing x86_64 (single file, gcc)..."
gcc -DSINGLE_FILE=1 -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86_64-gcc.o"
echo "Testing i386 (single file, gcc)..."
gcc -DSINGLE_FILE=1 -g -m32 -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86-gcc.o"
echo "Testing x86_64 (multi file)..."
clang -target x86_64-unknown-linux-gnu -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86_64-clang.o"
echo "Testing i386 (multi file)..."
clang -target i386-unknown-linux-gnu -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86-clang.o"
echo "Testing riscv32(multi file)..."
clang -target riscv32-unknown-linux-gnu -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_riscv32-clang.o"
echo "Testing riscv64(multi file)..."
clang -target riscv64-unknown-linux-gnu -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_riscv64-clang.o"
echo "Testing aarch64 (multi file)..."
clang -target aarch64-unknown-linux-gnu -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_aarch64-clang.o"
echo "Testing arm (multi file)..."
clang -target arm-unknown-linux-gnu -mfloat-abi=soft -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_arm32-clang.o"
echo "Testing x86_64 (multi file, gcc)..."
gcc -g -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86_64-gcc.o"
echo "Testing i386(multi file, gcc)..."
gcc -g -m32 -c "${SCRIPT_DIR}/test.c" -o "${SCRIPT_DIR}/test_x86-gcc.o"