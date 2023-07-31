#!/bin/bash

set -e

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
SOCK="$(mktemp -u).sock"
CONTAINER_NAME="demo-training-setup-tsffs"

nohup kitty -T demo -o font_size=28 -o allow_remote_control=yes \
    --listen-on "unix://${SOCK}" >/dev/null 2>&1 &
KITTY="$!"

cleanup() {
    kill "${KITTY}"
    rm "${SOCK}"
    docker stop "${CONTAINER_NAME}"
    docker container rm "${CONTAINER_NAME}"
}

trap cleanup EXIT
trap cleanup INT

read -p "Press enter to start: "

slow() {
    WPM=130
    SPC=$(bc -l <<<"60.0/(${WPM}*5.0)")
    JITTER=$(awk 'BEGIN{srand(); print 0.01+(rand()*0.08)}')
    WAIT=$(bc -l <<<"${SPC}+${JITTER}")
    sleep "${WAIT}"
}

krun() {
    echo "COMMAND:" "${@}"
    read -p "Press enter to continue: "
    for arg in "$@"; do
        for ((i = 0; i < ${#arg}; i++)); do
            kitty @ send-text --to "unix://${SOCK}" "${arg:$i:1}"
            slow
        done
        kitty @ send-text --to "unix://${SOCK}" " "
    done
    kitty @ send-text --to "unix://${SOCK}" "\n"
}

krun_silent() {
    read -p "Press enter to continue: "
    for arg in "$@"; do
        for ((i = 0; i < ${#arg}; i++)); do
            kitty @ send-text --to "unix://${SOCK}" "${arg:$i:1}"
            slow
        done
        kitty @ send-text --to "unix://${SOCK}" " "
    done
    kitty @ send-text --to "unix://${SOCK}" "\n"
}

krun_fast() {
    kitty @ send-text --to "unix://${SOCK}" "${@}"
    kitty @ send-text --to "unix://${SOCK}" "\n"
}

info() {
    echo "[*] $@"
}

info "Asking for kerberos password..."
read -p "Kerberos Password: " -s PASSWORD

info "Removing running container..."
docker stop "${CONTAINER_NAME}" || echo "Nothing to stop :)"
docker rm -f "${CONTAINER_NAME}" || echo "Nothing to remove :)"
docker container rm -f "${CONTAINER_NAME}" || echo "Nothing to remove :)"

info "Starting container..."
krun_fast docker run --name "${CONTAINER_NAME}" -v \
    "${SCRIPT_DIR}/../../../:/root/applications.security.fuzzing.confuse/" -it \
    ubuntu:22.04 bash

info "Waiting for container to start..."
sleep 5
PROMPT=$(docker exec -t "${CONTAINER_NAME}" bash -ic 'printf "${PS1@P}"')
krun_fast clear

krun apt-get -y update
krun apt-get -y install krb5-user smbclient git curl build-essential
krun AMR.CORP.INTEL.COM
# For some reason it asks for servers and administrative servers on 22.04
krun
krun
krun kinit "${USER}"
sleep 2
kitty @ send-text --to "unix://${SOCK}" "${PASSWORD}\n"
sleep 1
krun klist
krun curl --proto '=https' --tlsv1.2 -sSf 'https://sh.rustup.rs' '|' 'sh' -s -- -y
krun source '$HOME/.cargo/env'
krun cargo new --bin /tmp/test-rust
krun cargo run --manifest-path /tmp/test-rust/Cargo.toml
krun mkdir -p '$HOME/simics/'
krun curl -o '$HOME/simics/ispm-internal-latest-linux64.tar.gz' 'https://af02p-or.devtools.intel.com/artifactory/simics-repos/pub/simics-installer/intel-internal/ispm-internal-latest-linux64.tar.gz'
krun tar -C '$HOME/simics' -xvf '$HOME/simics/ispm-internal-latest-linux64.tar.gz'
krun '$HOME/simics/intel-simics-package-manager-1.7.3-intel-internal/ispm' \
    install \
    --install-dir '$HOME/simics/' \
    --package-repo 'https://af02p-or.devtools.intel.com/ui/native/simics-local/pub/simics-6/linux64/' \
    '1000-6.0.167' \
    '2096-6.0.68'
krun Y
krun cd '$HOME/applications.security.fuzzing.confuse'
krun cargo build --features=6.0.167
read -p "Press enter to finish"
