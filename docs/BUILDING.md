# Building

## Dependencies

There are a few dependencies, for AFL++ primarily.

```sh
$ sudo apt-get install build-essential python3-dev automake cmake git flex bison libglib2.0-dev libpixman-1-dev \
    python3-setuptools cargo libgtk-3-dev gcc-11 g++-11 gcc-11-plugin-dev libstdc++-11-dev \
    clang-14 clang-tools-14 libc++-14-dev:amd64 libc++-dev:amd64 libc++1:amd64 \
    libc++1-14:amd64 libc++abi-14-dev:amd64 libc++abi-dev:amd64 libc++abi1:amd64 \
    libc++abi1-14:amd64 libclang-14-dev libclang-common-14-dev libclang-cpp14 \
    libclang1-14 liblldb-14 liblldb-14-dev liblldb-dev:amd64 libllvm-14-ocaml-dev \
    libllvm14:amd64 lld-14 lldb-14 llvm-14 llvm-14-dev llvm-14-linker-tools \
    llvm-14-runtime llvm-14-tools python3-clang:amd64 python3-clang-14 python3-lldb-14
```


## Build

This project uses the meson build system and will be able to be built by running:

```sh
$ meson setup builddir
$ meson compile -C builddir
```

*NOTE*: If you installed llvm by specifying `llvm-14` instead of simply `llvm` (which
is the method you should most likely use), you will need to run `meson` with:

```sh
$ LLVM_CONFIG=$(which llvm-config-14) meson setup builddir
```