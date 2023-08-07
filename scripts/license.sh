#!/bin/bash

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if ! command -v fd &>/dev/null; then
    echo "fd must be installed! Install with 'cargo install fd-find'"
    exit 1
fi

if ! command -v rargs &>/dev/null; then
    echo "rargs must be installed! Install with 'cargo install rargs'"
    exit 1
fi

LICENSE_RUST="// Copyright (C) 2023 Intel Corporation\n\
// SPDX-License-Identifier: Apache-2.0\n"
LICENSE_PYTHON="# Copyright (C) 2023 Intel Corporation\n\
# SPDX-License-Identifier: Apache-2.0\n"
LICENSE_C="// Copyright (C) 2023 Intel Corporation\n\
// SPDX-License-Identifier: Apache-2.0\n"

fd -t f -0 -E 'simics-api-sys/src/bindings' '.*\.rs$' "${SCRIPT_DIR}/../" | rargs -0 bash -c \
    "if ! grep -q 'SPDX-License-Identifier: Apache-2.0' {}; then
        if grep -qzE '^#!/' {}; then
            echo 'Adding license to file with shebang' {}
            sed -i '2s/^/\n\/\/ Copyright (C) 2023 Intel Corporation\n\/\/ SPDX-License-Identifier: Apache-2.0\n\n/' {}
        else
            echo 'Adding license to ' {}
            sed -i '1s/^/\/\/ Copyright (C) 2023 Intel Corporation\n\/\/ SPDX-License-Identifier: Apache-2.0\n\n/' {}
        fi
    fi"
