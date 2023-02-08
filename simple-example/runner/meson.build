runner_src = 'testcode.c'
runner_include = [confuse_host_if_inc]

runner = executable(
    'runner',
    runner_src,
    include_directories: [runner_include],
    link_with: confuse_host_if_lib,
)