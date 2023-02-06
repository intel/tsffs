project('confuse', 'c', 'cpp')


# Build project
aflpp = subproject('AFLplusplus')
aflpp_outputs = aflpp.get_variable('aflpp_outputs')
aflpp_afl_clangpp = aflpp_outputs['afl-clang++']

subdir('confuse-host-if')
subdir('confuse-afl')