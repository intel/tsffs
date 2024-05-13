#!/bin/bash

set -e

ispm packages --list-installed --json | jq -r '.installedPackages[] | select(.pkgNumber == 1000) | .version' | tac | grep -v pre | sed -ne '/6.0.163/,$ p'