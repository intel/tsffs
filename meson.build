project('confuse', 'c', 'cpp')


# Build project
aflpp = subproject('AFLplusplus')
aflpp_afl_clangpp = aflpp.get_variable('aflpp_afl_clangpp')
subdir('confuse-afl')
# subdir('confuse-host-if')
