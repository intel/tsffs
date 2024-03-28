#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cp "${SCRIPT_DIR}/../../../harness/tsffs.h" "${SCRIPT_DIR}/tsffs.h"
cp "${SCRIPT_DIR}/../../rsrc/minimal_boot_disk.craff" "${SCRIPT_DIR}/minimal_boot_disk.craff"

ninja