#!/bin/bash

# Copyright (C) 2024 Intel Corporation
# SPDX-License-Identifier: Apache-2.0

if ! command -v jq &>/dev/null; then
    echo "jq must be installed. Install it with your package manager"
    exit 1
fi

if ! command -v git &>/dev/null; then
    echo "jq must be installed. Install it with your package manager"
    exit 1
fi

if [ -z "${GITHUB_TOKEN}" ]; then
    if ! command -v gh &>/dev/null; then
        echo "gh must be installed. Install it with your package manager"
        exit 1
    fi

    if ! GITHUB_TOKEN=$(gh auth token); then
        GITHUB_TOKEN=$(gh auth status -t 2>&1 | grep 'Token:' | awk '{print $3}') ||
            (echo "Failed to get token." && exit 1)

    fi

fi

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
DEPENDABOT_DIR="${SCRIPT_DIR}/../.github/dependabot"

pushd "${SCRIPT_DIR}" || exit 1

# REPOSITORY=$(git remote get-url origin | awk -F'/' '{print $(NF-1)"/"$(NF)}')
DATE=$(date '+%Y-%m-%d')
CSV="${DEPENDABOT_DIR}/${DATE}.csv"
JSON="${DEPENDABOT_DIR}/${DATE}.json"

curl -o "${JSON}" -L \
    -H "Accept: application/vnd.github+json" \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    -H "X-Github-Api-Version: 2022-11-28" \
    https://api.github.com/repos/intel/tsffs/dependabot/alerts

echo "CVE,Package Name,Severity,Manifest File,Status,CVSS,CVSS Vector,Vulnerable Versions,Fixed Versions,Triaged By,Triage Reason,Triage Comment" >"${CSV}"

jq '.[] | [.security_advisory.cve_id,.dependency.package.name,.security_advisory.severity,.dependency.manifest_path,.state,.security_advisory.cvss.score,.security_advisory.cvss.vector_string,.security_vulnerability.vulnerable_version_range,.fixed_at,.dismissed_by,.dismissed_reason,.dismissed_comment] | @csv' <"${JSON}" >>"${CSV}"

echo "🐱 Adding dependabot outputs to git with git add"

git add "${JSON}"
git add "${CSV}"
