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

src.json:
	$(info src.rs $@)
	$(SIMICS_PROJECT)/src.rs -o $@ \
		-f src.rs -f Cargo.toml -d src