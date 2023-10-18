# We are creating a module which registers the class `tsffs`
MODULE_CLASSES = tsffs
# The module is thread safe
THREAD_SAFE = yes
# This is passed as a dependency to the final shared object in Make, so this dependency is
# triggered to invoke cargo
EXTRA_OBJ_FILES = $(TARGET_DIR)/../obj/release/lib$(TARGET).a

ifeq ($(MODULE_MAKEFILE),)
$(error Make sure you compile your module from the project directory)
else
include $(MODULE_MAKEFILE)
endif

# Cargo generates a .d file as well as the library for us
$(TARGET_DIR)/../obj/release/lib$(TARGET).a $(TARGET_DIR)/../obj/release/lib$(TARGET).d:
	$(info CARGO $@)
	cargo rustc -r --manifest-path $(SRC_BASE)/tsffs/src/tsffs/Cargo.toml --target-dir $(TARGET_DIR)/../obj/ -- -C link-args="-Wl,--disable-new-dtags -Wl,-rpath,$(SIMICS_BASE)/$(HOST_TYPE)/bin:$(dir $(PYTHON3_LDFLAGS))"

# We include the .d file that cargo generates, which includes lib$(TARGET).a: [the list of dependencies]
include $(TARGET_DIR)/../obj/release/lib$(TARGET).d

# Set the user-init-local flag to make module_id.c invoke init_local instead of providing an
# empty one
_INIT_LOCAL_FLAG = --user-init-local