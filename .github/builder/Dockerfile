# hadolint global ignore=DL3033,SC3044
FROM novafacing/fedora-rustc-oldcompat:0.0.2

ENV PATH="${PATH}:/simics/ispm/"
ENV PATH="${PATH}:/root/.cargo/bin/"

COPY . /tsffs/

WORKDIR /tsffs/

RUN mkdir -p /simics/ispm && \
    tar -C /simics/ispm --strip-components=1 -xvf .github/builder/rsrc/ispm.tar.gz && \
    ispm settings install-dir /simics && \
    ispm packages --install-bundle .github/builder/rsrc/simics.ispm --non-interactive --trust-insecure-packages && \
    ispm projects "$(pwd)" --create --ignore-existing-files --non-interactive && \
    bin/project-setup --force && \
    cargo -Zscript build.rs
