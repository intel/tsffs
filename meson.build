project('confuse', 'c', 'cpp')

# Build AFL++

# We expect (on Ubuntu 22.04):
#
# build-essential python3-dev automake cmake git flex bison libglib2.0-dev libpixman-1-dev \
# python3-setuptools cargo libgtk-3-dev gcc-12-plugin-dev libstdc++-12-dev

# We also need:
# clang-14
# clang-tools-14
# libc++-14-dev:amd64
# libc++-dev:amd64
# libc++1:amd64
# libc++1-14:amd64
# libc++abi-14-dev:amd64
# libc++abi-dev:amd64
# libc++abi1:amd64
# libc++abi1-14:amd64
# libclang-14-dev
# libclang-common-14-dev
# libclang-cpp14
# libclang1-14
# liblldb-14
# liblldb-14-dev
# liblldb-dev:amd64
# libllvm-14-ocaml-dev
# libllvm14:amd64
# lld-14
# lldb-14
# llvm-14
# llvm-14-dev
# llvm-14-linker-tools
# llvm-14-runtime
# llvm-14-tools
# python3-clang:amd64
# python3-clang-14
# python3-lldb-14

aflpp_deps = [
    dependency('glib-2.0'),
    dependency('pixman-1'),
    dependency('gtk+-3.0'),
]

cc = meson.get_compiler('c')
cc.find_library('stdc++', required: true)
flex = find_program('flex', required: true)
bison = find_program('bison', required: true)
cargo = find_program('cargo', required: true)

aflpp_source_dir = meson.current_source_dir() / 'AFLplusplus'
aflpp_build_script = meson.current_source_dir() / 'buildafl.sh'

aflpp_binaries = [
    'afl-analyze',
    'afl-as',
    'afl-cc',
    'afl-cc.8',
    'afl-cmin',
    'afl-fuzz',
    'afl-gotcpu',
    'afl-ld-lto',
    'afl-persistent-config',
    'afl-plot',
    'afl-showmap',
    'afl-system-config',
    'afl-tmin',
    'afl-whatsup',
    'afl-wine-trace',
]

aflpp_libs = [
    'afl-gcc-cmplog-pass.so',
    'afl-gcc-cmptrs-pass.so',
    'afl-gcc-pass.so',
    'afl-llvm-dict2file.so',
    'afl-llvm-lto-instrumentlist.so',
    'afl-llvm-pass.so',
    'cmplog-instructions-pass.so',
    'cmplog-routines-pass.so',
    'cmplog-switches-pass.so',
    'compare-transform-pass.so',
    'libdislocator.so',
    'libnyx.so',
    'libtokencap.so',
    'SanitizerCoverageLTO.so',
    'SanitizerCoveragePCGUARD.so',
    'split-compares-pass.so',
    'split-switches-pass.so',
    'afl-compiler-rt-64.o',
    'afl-compiler-rt.o',
    'afl-llvm-rt-lto-64.o',
    'afl-llvm-rt-lto.o',
    'libAFLDriver.a',
    'libAFLQemuDriver.a',
]

# Symlinks AFL++ generates, these change invocation of binaries so they are somewhat important
aflpp_links = [
    'afl-c++.8',
    'afl-gcc-fast',
    'afl-g++-fast',
    'afl-g++',
    'afl-gcc-fast.8',
    'afl-clang-fast',
    'afl-clang++',
    'as',
    'afl-g++-fast.8',
    'afl-c++',
    'afl-gcc',
    'afl-clang-fast++',
    'afl-clang-fast.8',
    'afl-clang',
    'afl-clang-fast++.8',
]

aflpp_dir = 'AFLplusplus'

aflpp_target_bins = custom_target(
  'aflpp',
  input: [],
  output: [aflpp_binaries + aflpp_libs + aflpp_links],
  command: ['bash', aflpp_build_script, meson.current_build_dir(), aflpp_binaries + aflpp_libs + aflpp_links],
  build_by_default: true,
)

afl_clang = meson.current_build_dir() / 'afl-clang'
afl_clangpp = meson.current_build_dir() / 'afl-clang++'

# Build project

subdir('confuse-afl')
# subdir('confuse-host-if')
