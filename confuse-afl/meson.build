confuse_afl_src = [
    meson.current_source_dir() / 'confuse-afl-wrapper.cpp',
    meson.current_source_dir() / 'fake_harness.cpp',
]

# There's actually no way to switch compilers for one subdir, so there are
# a few ways to make this more 'correct':
#
# * Make this a subproject using CC=afl-clang CXX=afl-clang++
# * Use a fake cross-compilation setup
instrumented_test = custom_target(
    'instrumentedTest',
    input: aflpp_afl_clangpp,
    output: ['instrumentedTest'],
    command: [
        aflpp_afl_clangpp,
        '-g',
        '--std=c++11',
        '-I',
        meson.current_source_dir(),
        '-o',
        meson.current_build_dir() / 'instrumentedTest',
        confuse_afl_src
    ],
    build_by_default: true,
)

# afl_clangpp.process(confuse_afl_src, extra_args: [
#     '-g',
#     '--std=c++11',
#     '-I',
#     meson.current_source_dir(),
# ])