# coding: utf-8

# © 2010 Intel Corporation
#
# This software and the related documents are Intel copyrighted materials, and
# your use of them is governed by the express license under which they were
# provided to you ("License"). Unless the License provides otherwise, you may
# not use, modify, copy, publish, distribute, disclose or transmit this software
# or the related documents without Intel's prior written permission.
#
# This software and the related documents are provided as is, with no express or
# implied warranties, other than those that are expressly stated in the License.



USER_BUILD_ID=tsffs:1

# The build rule to build the fuzzer executable
$(SIMCS_PROJECT)/$(HOST_TYPE)/obj/release/fuzzer $(SIMICS_PROJECT)/$(HOST_TYPE)/obj/release/fuzzer.d:
	$(info CARGO $@)
	cargo rustc -r --manifest-path $(SIMICS_PROJECT)/src/fuzzer/Cargo.toml --target-dir $(SIMICS_PROJECT)/$(HOST_TYPE)/obj/ --bin fuzzer -- -C link-args="-Wl,--disable-new-dtags -Wl,-rpath,$(SIMICS_BASE)/$(HOST_TYPE)/bin:$(dir $(PYTHON3_LDFLAGS)) -Wl,-rpath-link,$(SIMICS_BASE)/$(HOST_TYPE)/bin;$(dir $(PYTHON3_LDFLAGS))"
	$(info MKDIR $(SIMICS_PROJECT)/$(HOST_TYPE)/bin/)
	mkdir -p $(SIMICS_PROJECT)/$(HOST_TYPE)/bin/
	$(info CP $(SIMICS_PROJECT)/$(HOST_TYPE)/obj/release/fuzzer $(SIMICS_PROJECT)/$(HOST_TYPE)/bin/fuzzer)
	cp $(SIMICS_PROJECT)/$(HOST_TYPE)/obj/release/fuzzer $(SIMICS_PROJECT)/$(HOST_TYPE)/bin/fuzzer

# We include the .d file that cargo generates, which includes lib$(TARGET).a: [the list of dependencies]
include $(SIMICS_PROJECT)/$(HOST_TYPE)/obj/release/fuzzer.d