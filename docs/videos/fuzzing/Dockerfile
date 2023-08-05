FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

ARG USERNAME
ARG PASSWORD
ARG GH_TOKEN

RUN apt-get -y update && \
    apt-get -y install krb5-user smbclient curl git build-essential gpg bash

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

COPY <<EOF /etc/krb5.conf
[libdefaults]
default_realm = AMR.CORP.INTEL.COM

[realms]

[domain_realm]

EOF

RUN echo "${PASSWORD}" | kinit "${USERNAME}" && \
    klist

RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | \
    gpg --dearmor -o /usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    | tee /etc/apt/sources.list.d/github-cli.list >/dev/null && \
    apt-get -y update && \
    apt-get -y install gh

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

RUN echo "${PASSWORD}" | kinit "${USERNAME}" && \
    mkdir -p "$HOME/simics" && \
    curl -o "$HOME/simics/ispm-internal-latest-linux64.tar.gz" \
    'https://af02p-or.devtools.intel.com/artifactory/simics-repos/pub/simics-installer/intel-internal/ispm-internal-latest-linux64.tar.gz' && \
    tar -C "$HOME/simics" -xvf "$HOME/simics/ispm-internal-latest-linux64.tar.gz" && \
    "$HOME/simics/intel-simics-package-manager-1.7.4-intel-internal/ispm" \
    install \
    --non-interactive \
    --install-dir "/root/simics/" \
    --package-repo 'https://af02p-or.devtools.intel.com/ui/native/simics-local/pub/simics-6/linux64/' \
    '1000-6.0.168' \
    '2096-6.0.69'

COPY --from=applications.security.fuzzing.confuse . /root/applications.security.fuzzing.confuse

WORKDIR /root/applications.security.fuzzing.confuse/

# RUN gh auth setup-git && \
#     gh repo clone intel-innersource/applications.security.fuzzing.confuse

# WORKDIR /root/applications.security.fuzzing.confuse

RUN echo "SIMICS_HOME=/root/simics/" > .env
#     "$HOME/.cargo/bin/cargo" build --release --features=6.0.168
