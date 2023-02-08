project('confuse', 'c', 'cpp')

fs = import('fs')
ls = find_program('ls', required: true)

# Get binaries
yq = find_program(meson.current_source_dir() / 'bin' / 'yq_linux_amd64', required: true)

# Get simics binaries and directory setup for workspace configuration
simics_home = get_option('simics_home')

if not fs.is_dir(simics_home)
    error('simics_home not set or does not exist. Pass -Dsimics_home=/path/to/simics/home')
endif


##################################################################################################
## Subproject setup & derived objects
##
##################################################################################################

# edk2 sources, these are not built sources because each EFI module likely needs its own build
# steps which may be complex. Builds for different EFI modules should be implemented in those
# modules, not here, and should use the edk2_src and edk2_libc_src generators to create a fresh
# copy of the sources before build. See `simple-example/HelloFuzzing/meson.build` as an example
# *DO NOT* use these objects directly, copy them. For example:
#
# hello_fuzzing_edk2_src = meson.current_build_dir() / 'edk2'
# hello_fuzzing_edk2_libc_src = meson.current_build_dir() / 'edk2-libc'
# 
# run_command(cp, '-a', edk2_src.full_path(), hello_fuzzing_edk2_src, check: true)
# run_command(cp, '-a', edk2_libc_src.full_path(), hello_fuzzing_edk2_libc_src, check: true)
# 
# It *is* ok to run `cp` at configure time, meson basically does it too
edk2 = subproject('edk2')
edk2_src_dir = edk2.get_variable('edk2_build_path')
edk2_libc_src_dir = edk2.get_variable('edk2_libc_build_path')

aflpp = subproject('AFLplusplus')
aflpp_outputs = aflpp.get_variable('aflpp_outputs')
# Add any depended-on commands from aflpp_outputs here, not where they are used. This will help
# make things cleaner.
aflpp_afl_clangpp = aflpp_outputs['afl-clang++']

subdir('confuse-host-if')
subdir('confuse-afl')

subdir('target-sw')
subdir('simple-example')
subdir('simics')