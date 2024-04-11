# hadolint global ignore=DL3033,SC3044
FROM fedora:20

ENV PATH="${PATH}:/root/.cargo/bin/"

COPY .github/builder/rsrc/rustup-init /install/rustup-init
COPY .github/builder/rsrc/make-4.4.1.tar.gz /install/make-4.4.1.tar.gz
COPY .github/builder/rsrc/cmake-3.28.0-rc5-linux-x86_64.tar.gz /install/cmake-3.28.0-rc5-linux-x86_64.tar.gz
COPY .github/builder/rsrc/lld-5.0.2.src.tar.xz /install/lld-5.0.2.src.tar.xz
COPY .github/builder/rsrc/cfe-5.0.2.src.tar.xz /install/cfe-5.0.2.src.tar.xz
COPY .github/builder/rsrc/llvm-5.0.2.src.tar.xz /install/llvm-5.0.2.src.tar.xz
COPY .github/builder/rsrc/rpms /install/rpms

RUN yum -y install /install/rpms/*.rpm && yum clean all

RUN chmod +x /install/rustup-init && \
    /install/rustup-init -y --default-toolchain nightly && \
    mkdir -p /make && \
    tar -C /make --strip-components=1 -xf /install/make-4.4.1.tar.gz && \
    pushd /make && \
    ./configure && \
    make && \
    make install && \
    make clean && \
    popd && \
    tar -C /usr/local/ --strip-components=1 -xf /install/cmake-3.28.0-rc5-linux-x86_64.tar.gz && \
    mkdir -p /llvm/tools/clang && \
    mkdir -p /llvm/tools/lld && \
    tar -C /llvm --strip-components=1 -xf /install/llvm-5.0.2.src.tar.xz && \
    tar -C /llvm/tools/clang --strip-components=1 -xf /install/cfe-5.0.2.src.tar.xz && \
    tar -C /llvm/tools/lld --strip-components=1 -xf /install/lld-5.0.2.src.tar.xz && \
    mkdir -p /llvm/build && \
    pushd /llvm/build && \
    cmake -G "Unix Makefiles" -DCMAKE_BUILD_TYPE="MinSizeRel" -DLLVM_TARGETS_TO_BUILD="X86" .. && \
    make -j "$(nproc)" && \
    make install && \
    make clean && \
    rm -rf /llvm/build/ && \
    popd && \
    rm -rf /make /llvm

WORKDIR /

ARG PUBLIC_SIMICS_PACKAGE_VERSION_1000
ENV PATH="${PATH}:/simics/ispm/"
ENV PATH="${PATH}:/root/.cargo/bin/"

COPY . /tsffs/

WORKDIR /tsffs/

RUN mkdir -p /simics/ispm && \
    tar -C /simics/ispm --strip-components=1 -xf .github/builder/rsrc/ispm.tar.gz && \
    ispm settings install-dir /simics && \
    ispm packages --install-bundle .github/builder/rsrc/simics.ispm --non-interactive --trust-insecure-packages

RUN RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=$(which ld.lld)" && \
    export RUSTFLAGS && \
    cargo install --path simics-rs/cargo-simics-build && \
    cargo simics-build -r && \
    mkdir -p /packages && \
    cp target/release/*.ispm /packages
