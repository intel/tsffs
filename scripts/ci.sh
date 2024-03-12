#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

#Run workflows locally using act

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
WORKFLOW_FILE="${SCRIPT_DIR}/../.github/workflows/ci.yml"
SECRETS_FILE="${SCRIPT_DIR}/../.secrets"

if [ ! -f "${SECRETS_FILE}" ]; then
    echo "No file '${SECRETS_FILE}' found. Please create one. It must have the following keys:
            GITHUB_TOKEN" \
        "You can find your GitHub token with 'gh auth token'"
    exit 1
fi

if ! command -v act &>/dev/null; then
    echo "act must be installed! Install at https://github.com/nektos/act"
    exit 1
fi

if ! command -v unbuffer &>/dev/null; then
    echo "unbuffer must be installed! Install 'expect' from your package manager"
    exit 1
fi

populate_env_file() {
    ENV_FILE="${1}"
    echo "Attempting automatic configuration of proxy with ENV_FILE=${ENV_FILE}"

    if [ -z "${HTTP_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTP_PROXY=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    elif [ -n "${HTTP_PROXY}" ]; then
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    fi
    echo "HTTP_PROXY=${HTTP_PROXY}" >>"${ENV_FILE}"
    echo "proxy=${HTTP_PROXY}" >>"${ENV_FILE}"

    if [ -z "${HTTPS_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTPS_PROXY=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    elif [ -n "${HTTPS_PROXY}" ]; then
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    fi
    echo "HTTPS_PROXY=${HTTPS_PROXY}" >>"${ENV_FILE}"

    if [ -z "${http_proxy}" ] && [ -f ~/.docker/config.json ]; then
        http_proxy=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config http_proxy=${http_proxy}"
    elif [ -n "${http_proxy}" ]; then
        echo "Exported docker config http_proxy=${http_proxy}"
    fi
    echo "http_proxy=${http_proxy}" >>"${ENV_FILE}"

    if [ -z "${https_proxy}" ] && [ -f ~/.docker/config.json ]; then
        https_proxy=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config https_proxy=${https_proxy}"
    elif [ -n "${https_proxy}" ]; then
        echo "Exported docker config https_proxy=${https_proxy}"
    fi
    echo "https_proxy=${https_proxy}" >>"${ENV_FILE}"

    if [ -z "${NO_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        NO_PROXY=$(grep noProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    elif [ -n "${NO_PROXY}" ]; then
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    fi
    echo "NO_PROXY=${NO_PROXY}" >>"${ENV_FILE}"

    cat "${ENV_FILE}"
}

ENV_FILE=$(mktemp)
ARTIFACT_DIR=$(mktemp -d)
populate_env_file "${ENV_FILE}"
mkdir -p "${SCRIPT_DIR}/../.github/logs/"
unbuffer act -W "${WORKFLOW_FILE}" --env-file="${ENV_FILE}" --secret-file="${SECRETS_FILE}" \
    --artifact-server-path "${ARTIFACT_DIR}" --artifact-server-addr "0.0.0.0" \
    "$@" | tee "${SCRIPT_DIR}/../.github/logs/$(date '+%F-%T').log"
rm "${ENV_FILE}"
