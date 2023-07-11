#!/bin/bash

# Run workflows locally using act

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
WORKFLOW_FILE="${SCRIPT_DIR}/.github/workflows/ci.yml"
ENV_FILE=$(mktemp -p /tmp .env.XXXXXXXX)

# Populate a .env file with proxy information to pass to act
populate_env_file() {
    ENV_FILE="${1}"
    echo "Running with ENV_FILE=${ENV_FILE}"

    if [ -z "${HTTP_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTP_PROXY=$(grep httpProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    elif [ ! -z "${HTTP_PROXY}" ]; then
        HTTP_PROXY="${HTTP_PROXY}"
        echo "Exported docker config HTTP_PROXY=${HTTP_PROXY}"
    fi
    echo "HTTP_PROXY=${HTTP_PROXY}" >> "${ENV_FILE}"

    if [ -z "${HTTPS_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        HTTPS_PROXY=$(grep httpsProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    elif [ ! -z "${HTTPS_PROXY}" ]; then
        HTTPS_PROXY="${HTTPS_PROXY}"
        echo "Exported docker config HTTPS_PROXY=${HTTPS_PROXY}"
    fi
    echo "HTTPS_PROXY=${HTTPS_PROXY}" >> "${ENV_FILE}"

    if [ -z "${NO_PROXY}" ] && [ -f ~/.docker/config.json ]; then
        NO_PROXY=$(grep noProxy ~/.docker/config.json | awk -F'\"[:space:]*:[:space:]*' '{split($2,a,"\""); print a[2]}')
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    elif [ ! -z "${NO_PROXY}" ]; then
        NO_PROXY="${NO_PROXY}"
        echo "Exported docker config NO_PROXY=${NO_PROXY}"
    fi
    echo "NO_PROXY=${NO_PROXY}" >> "${ENV_FILE}"

    cat "${ENV_FILE}"
}

if ! command -v act &> /dev/null; then
    echo "act must be installed! Install at https://github.com/nektos/act"
    exit 1
fi


populate_env_file "${ENV_FILE}"
act -W "${WORKFLOW_FILE}" --env-file="${ENV_FILE}" $@
rm "${ENV_FILE}"