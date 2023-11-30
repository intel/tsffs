# hadolint global ignore=DL3033,SC3044
FROM novafacing/fedora-rustc-oldcompat:0.0.1

ENV PATH="${PATH}:/simics/ispm/"
ENV PATH="${PATH}:/root/.cargo/bin/"

COPY .github/builder/rsrc/ispm.tar.gz /rsrc/ispm.tar.gz
COPY .github/builder/rsrc/simics.ispm /rsrc/simics.ispm

RUN mkdir -p /simics/ispm && \
    tar -C /simics/ispm --strip-components=1 -xvf /rsrc/ispm.tar.gz && \
    ispm settings install-dir /simics && \
    ispm packages --install-bundle /rsrc/simics.ispm --non-interactive --trust-insecure-packages && \
    ispm projects "$(pwd)" --create --ignore-existing-files --non-interactive && \
    bin/project-setup --force && \
    cargo -Zscript build.rs
