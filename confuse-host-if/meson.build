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

confuse_host_if_lib_dir = meson.current_build_dir()

confuse_host_if_inc_dir = meson.current_source_dir() / 'include'