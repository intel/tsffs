FROM fedora:20

ENV PATH="${PATH}:/simics/ispm/"
ENV PATH="${PATH}:/root/.cargo/bin/"

RUN yum -y update && \
    yum -y install \
        coreutils \
        gcc \
        gcc-c++ \
        make

COPY . /tsffs

WORKDIR /tsffs

RUN /tsffs/builder/rsrc/rustup-init -y --default-toolchain nightly

RUN mkdir -p /make && \
    tar -C /make --strip-components=1 -xvf /tsffs/builder/rsrc/make-4.4.1.tar.gz && \
    pushd /make && \
    ./configure && \
    make && \
    make install && \
    make clean && \
    popd

RUN tar -C /usr/local/ --strip-components=1 -xvf /tsffs/builder/rsrc/cmake-3.28.0-rc5-linux-x86_64.tar.gz && \
    cmake --version

RUN mkdir -p /llvm/tools/clang && \
    tar -C /llvm --strip-components=1 -xvf /tsffs/builder/rsrc/llvm-5.0.2.src.tar.xz && \
    tar -C /llvm/tools/clang --strip-components=1 -xvf /tsffs/builder/rsrc/cfe-5.0.2.src.tar.xz && \
    mkdir -p /llvm/build && \
    pushd /llvm/build && \
    cmake -G "Unix Makefiles" -DCMAKE_BUILD_TYPE="MinSizeRel" -DLLVM_TARGETS_TO_BUILD="X86" .. && \
    make -j$(nproc) && \
    make install && \
    make clean && \
    rm -rf /llvm/build/ && \
    popd

RUN mkdir -p /simics/ispm && \
    tar -C /simics/ispm --strip-components=1 -xvf /tsffs/builder/rsrc/ispm.tar.gz && \
    ispm settings install-dir /simics && \
    ispm packages --install-bundle /tsffs/builder/rsrc/simics.ispm --non-interactive --trust-insecure-packages && \
    ispm projects $(pwd) --create --ignore-existing-files --non-interactive && \
    bin/project-setup --force



    
