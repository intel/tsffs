#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cat <<EOF > "${SCRIPT_DIR}/tsffs.h"
// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

#if defined(__GNUC__) || defined(__clang__)
#ifdef __i386__
$(cat "${SCRIPT_DIR}/tsffs-gcc-x86.h")
#elif __x86_64__
$(cat "${SCRIPT_DIR}/tsffs-gcc-x86_64.h")
#elif __riscv && !__LP64__
$(cat "${SCRIPT_DIR}/tsffs-gcc-riscv32.h")
#elif __riscv && __LP64__
$(cat "${SCRIPT_DIR}/tsffs-gcc-riscv64.h")
#elif __aarch64__
$(cat "${SCRIPT_DIR}/tsffs-gcc-aarch64.h")
#elif __arm__
$(cat "${SCRIPT_DIR}/tsffs-gcc-arm32.h")
#else
#error "Unsupported platform!"
#endif
#elif _MSC_VER
$(cat "${SCRIPT_DIR}/tsffs-msvc-x86_64.h")
#else
#error "Unsupported compiler!"
#endif
EOF
