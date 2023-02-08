confuse_host_if_inc = include_directories('include')

confuse_host_if_src = [
    'confuse_dio.c',
    'confuse_ll.c',
]

confuse_host_if_lib = static_library(
    'confuse',
    confuse_host_if_src,
    include_directories: [confuse_host_if_inc],
)