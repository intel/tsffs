# -*- Makefile ; coding: utf-8 -*-

# (optional) Module-specific parameters for the DML dependency generator and compiler
DMLC_FLAGS :=

# (optional) space- or colon-separated list of module names (optionally followed by a subdirectory:
# modulename/some/path). The source code directory of this module (or the corresponding 
# subdirectory) will automatically be added to make's VPATH. The directories will also be added to
# the include search path for DML and C modules (using the -I options to the compilers).
#
# The current project as well as any installed Simics packages will be searched for the module
# source code. Note that only the first matching source code directory found will be
# used, allowing the project to override the version of an installed package.
EXTRA_MODULE_VPATH :=

# (optional) Additional object files to link into the module. The module build will be
# dependent on these files, so additional rules can be provided in the module's
# Makefile to build these files before linking them.
EXTRA_OBJ_FILES :=

# (optional) Directories to add to VPATH when building.
EXTRA_VPATH :=

# Header files to compile for an interface module. If IFACE_FILES contains
# file-name.h, in a Simics module named module-name, then the Python module
# simmod.module_name.file_name will be created, containing the Python bindings for all
# interfaces and types defined in file-name.h. The filenames in IFACE_FILES must not
# include any directory components; if any files are to be found in another directory,
# then that directory must be included in VPATH so the file is found correctly; e.g.,
# by setting the EXTRA_MODULE_VPATH variable.
# 
# See also section Restrictions in Defining New Interface Types in Model Builder User's
# Guide for restrictions and usage of Python bindings.
IFACE_FILES :=

# (optional) Parameters for the C and C++ preprocessor and compiler
MODULE_CFLAGS := 

# Space-separated list of configuration classes that this module registers. This
# information is used by SIM_get_class() to determine that this module should be
# automatically loaded when one of the listed classes is requested.
MODULE_CLASSES := tsffs_module

# Space-separated list of components that this module registers.
MODULE_COMPONENTS :=

# (optional) Module-specific flags for the C and C++ linker. Any particular library
# linking (such as -lpthread, -lm, or -L to give paths to search for libraries) can
# be included here. If the module has dependencies on LD_MODULE_PATH, it is possible
# to specify -rpath so that the module will contain the paths that were valid at
# compilation time.
MODULE_LDFLAGS := -Wl,-rpath,$(strip $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST)))))/../../lib/ -L $(strip $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST)))))/../../lib/ -ltsffs_module

# Space-separated list of Python source files to include. These Python files will be
# copied and potentially compiled (see COMPILE_PYC) and placed in a Python package
# specific to the module. If a module my-module includes the file file.py in
# PYTHON_FILES, then the resulting Python module will be available as
# simmod.my_module.file in Simics. Two filenames get special treatment if included in
# PYTHON_FILES:
#   - simics_start.py is automatically imported while Simics is launched.
#   - module_load.py is imported by Simics when the Simics module is loaded.
# The names checkpoint_update.py and __init__.py are reserved for future use, and not
# allowed in the list of files.
PYTHON_FILES := simics_start.py module_load.py

# (optional) User supplied free-text string describing the module version. The version
# string is available in Simics even without loading the module, through the
# list-modules command or the SIM_get_all_modules API function.
MODULE_USER_VERSION :=

# (optional) Simics API to use when compiling the module. See the Simics Migration
# Guide for a description on how to compile old modules with a new Simics version.
# Valid API settings are listed in [simics]/[host]/include/api-versions.mk.
SIMICS_API :=

# Source files to compile in the module. C source file names must end in .c; C++
# source file names must end in .cc, .cpp, .cxx or .C (the last not allowed on
# Windows). DML file names must have a .dml suffix. Any .py files should be listed in
# the PYTHON_FILES variable. For backward compatibility, modules using Simics API 4.8
# or older may still list Python files in the SRC_FILES variable.
SRC_FILES := tsffs_module.c

# (optional) Images to copy directly in the images subdirectory in $(TARGET_DIR)
SRC_IMAGES :=

# If set to 'yes', provides compiler and linker flags that allow building the SystemC
# Library adapter and SystemC devices. See the SystemC Library Programming Guide for
# more information.
SYSTEMC :=

# (optional) Parameters for the C and C++ preprocessor and compiler when using
# user-specified SystemC source.
SYSTEMC_CORE_CFLAGS :=

# (optional) SystemC core specific flags for the C and C++ linker. Any particular
# library linking can be included here.
SYSTEMC_CORE_LDFLAGS :=

# (optional) Parameters for the C and C++ preprocessor and compiler when compiling
# SystemC modules.
SYSTEMC_MODULE_CFLAGS :=

# If set to yes, declare that the module is thread-safe.
THREAD_SAFE := yes

# If set to 'yes', enable support for the default C++ Device API. It can also be set
# to a specific version to select which version of C++ Device API to use. Current
# supported versions are '1' and '2'. See the C++ Device API Programming Guide for
# more information.
USE_CC_API :=

# If set to 'yes', suppress output of the .xml device info file by the DML compiler.
SUPRESS_DEVICE_INFO :=

# An optional list of compatible compilers, in order of preference. The allowed list
# element values are gcc and cl, for MinGW and Visual Studio respectively. On
# platforms other than Windows, cl is ignored.
COMPILERS :=

# If this is set to 1, then the files listed in PYTHON_FILES are compiled, not copied.
COMPILE_PYC := 0

ifeq ($(MODULE_MAKEFILE),)
$(error Make sure you compile your module from the project directory)
else
include $(MODULE_MAKEFILE)
endif
