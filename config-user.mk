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

ifeq ($(HOST_TYPE),win64)
	LIBS=-lws2_32 -loleaut32 -lole32 -lbcrypt -luserenv -lntdll
endif

ifeq ($(HOST_TYPE),linux64)
	LDFLAGS=-Wl,--gc-sections
endif