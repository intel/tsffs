#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

set -e

if [ -z "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE is not set, defaulting to latest."
    SIMICS_BASE="$(ispm packages --list-installed --json | jq -r '[ .installedPackages[] | select(.pkgNumber == 1000) ] | ([ .[].version ] | max_by(split(".") | map(tonumber))) as $m | first(first(.[]|select(.version == $m)).paths[0])')"
    export SIMICS_BASE
fi

if [ ! -d "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE ${SIMICS_BASE} is not a directory."
    exit 1
fi

for TARGET in *; do
    if [ -d "${TARGET}" ]; then
        pushd "${TARGET}" || exit 1
        echo "Building ${TARGET}"
        ./build.sh
        popd || exit 1
    fi
done
