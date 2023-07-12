#!/bin/bash

#Run workflows locally using act

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
WORKFLOW_FILE="${SCRIPT_DIR}/../.github/workflows/ci.yml"
SECRETS_FILE="${SCRIPT_DIR}/../.secrets"

if [ ! -f "${SECRETS_FILE}" ]; then
    echo "No file '${SECRETS_FILE}' found. Please create one." \
        "If you are an Intel employee, you can find decryption keys at https://wiki.ith.intel.com/display/Simics/Simics+6."
    exit 1
fi

if !command -v act &>/dev/null; then
    echo "act must be installed! Install at https://github.com/nektos/act"
    exit 1
fi

populate_env_file() {
    ENV_FILE="${1}"
    echo "Attempting automatic configuration of proxy with ENV_FILE=${ENV_FILE}"

    if [ -z "${HTTP_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTP_PROXY=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    elif [ ! -z "${HTTP_PROXY}" ]; then
        HTTP_PROXY="${HTTP_PROXY}"
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    fi
    echo "HTTP_PROXY=${HTTP_PROXY}" >>"${ENV_FILE}"

    if [ -z "${HTTPS_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTPS_PROXY=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    elif [ ! -z "${HTTPS_PROXY}" ]; then
        HTTPS_PROXY="${HTTPS_PROXY}"
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    fi
    echo "HTTPS_PROXY=${HTTPS_PROXY}" >>"${ENV_FILE}"

    if [ -z "${http_proxy}" ] && [ -f ~/.docker/config.json ]; then
        http_proxy=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config http_proxy=${http_proxy}"
    elif [ ! -z "${http_proxy}" ]; then
        http_proxy="${http_proxy}"
        echo "Exported docker config http_proxy=${http_proxy}"
    fi
    echo "http_proxy=${http_proxy}" >>"${ENV_FILE}"

    if [ -z "${https_proxy}" ] && [ -f ~/.docker/config.json ]; then
        https_proxy=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config https_proxy=${https_proxy}"
    elif [ ! -z "${https_proxy}" ]; then
        https_proxy="${https_proxy}"
        echo "Exported docker config https_proxy=${https_proxy}"
    fi
    echo "https_proxy=${https_proxy}" >>"${ENV_FILE}"

    if [ -z "${HTTP_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTP_PROXY=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    elif [ ! -z "${HTTP_PROXY}" ]; then
        proxy="${HTTP_PROXY}"
        echo "Exported docker config proxy=${HTTP_PROXY}"
    fi
    echo "proxy=${HTTP_PROXY}" >>"${ENV_FILE}"

    if [ -z "${NO_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        NO_PROXY=$(grep noProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    elif [ ! -z "${NO_PROXY}" ]; then
        NO_PROXY="${NO_PROXY}"
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    fi
    echo "NO_PROXY=${NO_PROXY}" >>"${ENV_FILE}"

    cat "${ENV_FILE}"
}

ENV_FILE=$(mktemp)
populate_env_file "${ENV_FILE}"
docker pull amr-registry.caas.intel.com/1source/github-actions-runner:v2.304.0-ubuntu-20.04
act -W "${WORKFLOW_FILE}" --env-file="${ENV_FILE}" --secret-file="${SECRETS_FILE}" \
    -P gasp=github-actions-runner:v2.304.0-ubuntu-20.04 \
    -P self-hosted=github-actions-runner:v2.304.0-ubuntu-20.04 \
    $@
rm "${ENV_FILE}"
