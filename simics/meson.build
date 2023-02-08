bash = find_program('bash', required: true)
echo = find_program('echo', required: true)
mkdir = find_program('mkdir', required: true)
simics_manifests_dir = simics_home / 'manifests'

if not fs.is_dir(simics_manifests_dir)
    error('No "manifests" directory found in simics_home. Is simics installed correctly?')
endif

simics_manifests = run_command(
    ls,
    '-1',
    simics_manifests_dir,
    check: true
).stdout().strip().split()

simics_max_version = '0.0.0'
simics_max_manifest_file = ''
foreach simics_manifest : simics_manifests
    simics_base_version = run_command(
        yq,
        '.packages[] | select(.description == "Simics Base") | .version',
        simics_manifests_dir / simics_manifest,
        check: true,
    ).stdout().strip()

    if simics_base_version.version_compare('>' + simics_max_version)
        simics_max_version = simics_base_version
        simics_max_manifest_file = simics_manifest
    endif
endforeach

if simics_max_version == '0.0.0'
    error('No simics version found. Is simics installed correctly?')
endif

simics_manifest_file = simics_max_manifest_file
simics_base_version = simics_max_version
simics_qsp_x86_version = run_command(
    yq,
    '.packages[] | select(.description == "Quick-Start Platform") | .version',
    simics_manifests_dir / simics_manifest_file,
    check: true,
).stdout().strip()

simics_base_dir = simics_home / 'simics-' + simics_base_version
simics_qsp_x86_dir = simics_home / 'simics-qsp-x86-' + simics_qsp_x86_version
simics_base_bin_dir = simics_base_dir / 'bin'

simics_base_project_setup = find_program(simics_base_bin_dir / 'project-setup', required: true)

simics_confuse_project = meson.current_build_dir()
simics_confuse_project_package_list = simics_confuse_project / '.package-list'

run_command(
    simics_base_project_setup,
    '--ignore-existing-files',
    simics_confuse_project,
    check: true
)

simics_bin_dir = simics_confuse_project / 'bin'
simics_project_setup = find_program(simics_bin_dir / 'project-setup', required: true)

run_command(
    bash,
    '-c',
    'echo'
    + ' ' + simics_qsp_x86_dir
    + ' ' + '>'
    + ' ' + simics_confuse_project_package_list,
    check: true
)

run_command(simics_project_setup, check: true)

hello_fuzzing_efi_copy_cmd = custom_target(
    'hello_fuzzing_efi_copy_cmd',
    input: [hello_fuzzing_build_tgt],
    output: ['HelloFuzzing.efi'],
    command: [
        cp,
        '@INPUT@',
        simics_confuse_project,
    ],
    build_by_default: true,
)

simics_module_dirs = [
    'modules',
    'modules' / 'afl-branch-tracer',
    'modules' / 'confuse_dio',
    'modules' / 'confuse_dio-interface',
    'modules' / 'confuse_ll',
    'targets',
    'targets' / 'qsp-x86-fuzzing',
    'targets' / 'qsp-x86-fuzzing' / 'images',
    'simics-scripts',
    'simics-scripts' / 'blueprint',
]

simics_module_src = [
    'modules' / 'afl-branch-tracer' / 'afl-branch-tracer.c',
    'modules' / 'afl-branch-tracer' / 'Makefile',
    'modules' / 'confuse_dio' / 'confuse-dio.c',
    'modules' / 'confuse_dio' / 'Makefile',
    'modules' / 'confuse_dio' / 'module_load.py',
    'modules' / 'confuse_dio' / 'simics_start.py',
    'modules' / 'confuse_dio-interface' / 'confuse_dio-interface.dml',
    'modules' / 'confuse_dio-interface' / 'confuse_dio-interface.h',
    'modules' / 'confuse_dio-interface' / 'Makefile',
    'modules' / 'confuse_ll' / 'confuse-ll.c',
    'modules' / 'confuse_ll' / 'Makefile',
    'targets' / 'qsp-x86-fuzzing' / 'qsp-x86-uefi-app.yml',
    'targets' / 'qsp-x86-fuzzing' / 'run-uefi-app.simics',
    'targets' / 'qsp-x86-fuzzing' / 'images' / 'minimal_boot_disk.craff',
    'targets' / 'qsp-x86-fuzzing' / 'images' / 'run_uefi_app.nsh',
    # These aren't strictly "inputs" but they are needed to run
    'simics-scripts' / 'qsp-x86-uefi-app.py',
    'simics-scripts' / 'qsp-x86-uefi-app.yml',
    'simics-scripts' / 'blueprint' / 'testme.yml',
    'simics-scripts' / 'blueprint' / 'uefi-app-blueprint.include',
]

foreach dir : simics_module_dirs
    run_command(mkdir, '-p', simics_confuse_project / dir, check: true)
endforeach

foreach src_file : simics_module_src
    run_command(cp, meson.current_source_dir() / src_file, simics_confuse_project / src_file, check: true)
endforeach

confuse_project_build = custom_target(
    'confuse_project_build',
    input: [hello_fuzzing_efi_copy_cmd, simics_module_src],
    output: ['dummy-do-not-depend-on-this'],
    command: [
        make,
        '-C',
        simics_confuse_project,
    ],
    build_by_default: true,
)

