#!/bin/bash

set -e

cd "${HELLO_FUZZING_EDK2_DIR}"

make -C "${HELLO_FUZZING_EDK2_BASE_TOOLS}"

source "${HELLO_FUZZING_EDK2_EDKSETUP_SH}"

export PACKAGES_PATH="${HELLO_FUZZING_EDK2_LIBC_DIR}"

# Set target contents
cp "${HELLO_FUZZING_EDK2_CONF_TARGET_TXT_SRC}" "${HELLO_FUZZING_EDK2_CONF_TARGET_TXT}"
# Set AppPkg.dsc contents
cp "${HELLO_FUZZING_EDK2_APP_PKG_DSC_SRC}" "${HELLO_FUZZING_EDK2_APP_PKG_DSC}"

cd "${HELLO_FUZZING_EDK2_LIBC_DIR}"

build -p AppPkg/AppPkg.dsc -m AppPkg/Applications/HelloFuzzing/HelloFuzzing.inf

cp "${HELLO_FUZZING_EDK2_BUILD_DIR}/HelloFuzzing.efi" "${MESON_CURRENT_BUILD_DIR}"
cp "${HELLO_FUZZING_EDK2_BUILD_DIR}/HelloFuzzing.debug" "${MESON_CURRENT_BUILD_DIR}"



