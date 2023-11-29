#!/bin/bash

if [ -z "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE is not set, defaulting to latest."
    export SIMICS_BASE=$(ispm packages --list-installed --json | jq -r '[ .installedPackages[] | select(.pkgNumber == 1000) ] | ([ .[].version ] | max_by(split(".") | map(tonumber))) as $m | first(first(.[]|select(.version == $m)).paths[0])')
fi

if [ ! -d "${SIMICS_BASE}" ]; then
    echo "SIMICS_BASE ${SIMICS_BASE} is not a directory."
    exit 1
fi

for TARGET in *; do
    if [ -d "${TARGET}" ]; then
        pushd "${TARGET}" || exit 1
        ./build.sh
        popd
    fi
done
