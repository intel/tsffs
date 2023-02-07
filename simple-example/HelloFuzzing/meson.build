fs = import('fs')
make = find_program('make', required: true)
cat = find_program('cat', required: true)
bash = find_program('bash', required: true)
cp = find_program('cp', required: true)
mkdir = find_program('mkdir', required: true)

hello_fuzzing_src = [
    meson.current_source_dir() / 'HelloFuzzing.inf',
    meson.current_source_dir() / 'magic-instruction.h',
    meson.current_source_dir() / 'Hello.c',
]

hello_fuzzing_edk2_src = meson.current_build_dir() / 'edk2'
hello_fuzzing_edk2_libc_src = meson.current_build_dir() / 'edk2-libc'
hello_fuzzing_edk2_conf_target_txt = hello_fuzzing_edk2_src / 'Conf' / 'target.txt'
hello_fuzzing_edk2_app_pkg_dsc = hello_fuzzing_edk2_libc_src / 'AppPkg' / 'AppPkg.dsc'
hello_fuzzing_edk2_base_tools = hello_fuzzing_edk2_src / 'BaseTools'
hello_fuzzing_edk2_edksetup_sh = hello_fuzzing_edk2_src / 'edksetup.sh'
hello_fuzzing_build_dir = hello_fuzzing_edk2_libc_src / 'AppPkg' / 'Applications' / 'HelloFuzzing'
hello_fuzzing_magic_pipe_lib_build_dir = hello_fuzzing_edk2_libc_src / 'MagicPipeLib'
hello_fuzzing_edk2_build_dir = hello_fuzzing_edk2_src / 'Build' / 'AppPkg' / 'DEBUG_GCC5' / 'X64'
hello_fuzzing_edk2_app_pkg_dsc_src = meson.current_source_dir() / 'AppPkg.dsc'
hello_fuzzing_edk2_conf_target_txt_src = meson.current_source_dir() / 'target.txt'


if not fs.is_dir(hello_fuzzing_edk2_src)
    run_command(cp, '-a', edk2_src_dir, hello_fuzzing_edk2_src, check: true)
endif

if not fs.is_dir(hello_fuzzing_edk2_libc_src)
    run_command(cp, '-a', edk2_libc_src_dir, hello_fuzzing_edk2_libc_src, check: true)
endif

if not fs.is_dir(hello_fuzzing_build_dir)
    run_command(mkdir, '-p', hello_fuzzing_build_dir, check: true)

    foreach src_file : hello_fuzzing_src
        run_command(cp, src_file, hello_fuzzing_build_dir, check: true)
    endforeach


endif

if not fs.is_dir(hello_fuzzing_magic_pipe_lib_build_dir)
    run_command(mkdir, '-p', hello_fuzzing_magic_pipe_lib_build_dir, check: true)

    foreach src_file : magic_pipe_lib_src
        run_command(cp, magic_pipe_lib_src_dir / src_file, hello_fuzzing_magic_pipe_lib_build_dir / src_file, check: true)
    endforeach

    run_command(mkdir, '-p', hello_fuzzing_magic_pipe_lib_build_dir / 'Include', check: true)

    foreach src_file : magic_pipe_lib_include
        run_command(cp, magic_pipe_lib_src_dir / 'Include' / src_file, hello_fuzzing_magic_pipe_lib_build_dir / 'Include' / src_file, check: true)
    endforeach
endif


hello_fuzzing_build_tgt = custom_target(
    'hello_fuzzing_build_tgt',
    input: [],
    output: ['HelloFuzzing.efi', 'HelloFuzzing.debug'],
    command: [
        'build.sh',
    ],
    env: {
        'HELLO_FUZZING_EDK2_CONF_TARGET_TXT': hello_fuzzing_edk2_conf_target_txt,
        'HELLO_FUZZING_EDK2_APP_PKG_DSC': hello_fuzzing_edk2_app_pkg_dsc,
        'HELLO_FUZZING_EDK2_BASE_TOOLS': hello_fuzzing_edk2_base_tools,
        'HELLO_FUZZING_EDK2_EDKSETUP_SH': hello_fuzzing_edk2_edksetup_sh,
        'HELLO_FUZZING_EDK2_DIR': hello_fuzzing_edk2_src,
        'HELLO_FUZZING_EDK2_LIBC_DIR': hello_fuzzing_edk2_libc_src,
        'HELLO_FUZZING_EDK2_BUILD_DIR': hello_fuzzing_edk2_build_dir,
        'HELLO_FUZZING_EDK2_CONF_TARGET_TXT_SRC': hello_fuzzing_edk2_conf_target_txt_src,
        'HELLO_FUZZING_EDK2_APP_PKG_DSC_SRC': hello_fuzzing_edk2_app_pkg_dsc_src,
        'MESON_CURRENT_BUILD_DIR': meson.current_build_dir(),
    },
    build_by_default: true,
)
