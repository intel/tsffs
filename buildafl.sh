#!/bin/bash

OUTDIR="${1}"
AFLPP_BINS=("${@:2}")
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

if [ ! -d "${OUTDIR}" -o -z "${AFLPP_BINS}" ]; then
    echo "Usage: ${0} <outdir> <aflpp_bin...>"
    exit 1
fi

echo "Ensuring AFL++ is checked out"
cd "${SCRIPT_DIR}"
git submodule update --init

echo "Building AFL++"
cd "${SCRIPT_DIR}/AFLplusplus"
make source-only

echo "Built AFL++"

for BIN in "${AFLPP_BINS[@]}"; do
    echo "Copying ${BIN}"
    cp -a "${SCRIPT_DIR}/AFLplusplus/${BIN}" "${OUTDIR}"
done