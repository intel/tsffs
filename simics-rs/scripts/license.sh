#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

if ! command -v fd &>/dev/null; then
    echo "fd must be installed! Install with 'cargo install fd-find'"
    exit 1
fi

if ! command -v rargs &>/dev/null; then
    echo "rargs must be installed! Install with 'cargo install rargs'"
    exit 1
fi

fd -t f -0 '.*\.rs$' "${SCRIPT_DIR}/../" | rargs -0 bash -c \
    "if ! grep -q 'SPDX-License-Identifier: Apache-2.0' {}; then
        if grep -qzE '^#!/' {}; then
            echo 'Adding license to file with shebang' {}
            sed -i '2s/^/\n\/\/ Copyright (C) 2024 Intel Corporation\n\/\/ SPDX-License-Identifier: Apache-2.0\n\n/' {}
        else
            echo 'Adding license to ' {}
            sed -i '1s/^/\/\/ Copyright (C) 2024 Intel Corporation\n\/\/ SPDX-License-Identifier: Apache-2.0\n\n/' {}
        fi
    fi"

fd -t f -0 '.*\.(c|h|cc|hh|hpp|cpp)$' "${SCRIPT_DIR}/../" | rargs -0 bash -c \
    "if ! grep -q 'SPDX-License-Identifier: Apache-2.0' {}; then
        sed -i '1s/^/\/\/ Copyright (C) 2024 Intel Corporation\n\/\/ SPDX-License-Identifier: Apache-2.0\n\n/' {}
    fi"

fd -t f -0 '.*\.(c|h|cc|hh|hpp|cpp)$' "${SCRIPT_DIR}/../" | rargs -0 bash -c \
    "if ! grep -q 'SPDX-License-Identifier: Apache-2.0' {}; then
        sed -i '1s/^/# Copyright (C) 2024 Intel Corporation\n\# SPDX-License-Identifier: Apache-2.0\n\n/' {}
    fi"

MISSING_LICENSE_FILES=()

while IFS= read -r -d $'\0' LICENSE_REQUIRED_FILE; do
    if ! grep -q 'SPDX-License-Identifier: Apache-2.0' "${LICENSE_REQUIRED_FILE}"; then
        MISSING_LICENSE_FILES+=("${LICENSE_REQUIRED_FILE}")
    fi
done < <(fd -0 -t f -e 'c' -e 'dml' -e 'h' \
    -e 'inf' -e 'ini' -e 'ninja' -e 'nsh' -e 'py' -e 'rs' -e 'sh' -e 'simics' \
    -e 'toml' -e 'yaml' -e 'yml' -e 'cpp' -e 'cc' -e 'hh' -e 'hpp' . "${SCRIPT_DIR}/../")

if [ "${#MISSING_LICENSE_FILES[@]}" -eq 0 ]; then
    exit 0
else
    echo "Files found missing license block:"
    for MISSING_LICENSE_FILE in "${MISSING_LICENSE_FILES[@]}"; do
        echo "${MISSING_LICENSE_FILE}"
    done
    exit 1
fi
