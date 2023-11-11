# -*- Makefile ; coding: utf-8 -*-

# © 2015 Intel Corporation
#
# This software and the related documents are Intel copyrighted materials, and
# your use of them is governed by the express license under which they were
# provided to you ("License"). Unless the License provides otherwise, you may
# not use, modify, copy, publish, distribute, disclose or transmit this software
# or the related documents without Intel's prior written permission.
#
# This software and the related documents are provided as is, with no express or
# implied warranties, other than those that are expressly stated in the License.

# Select compiler by changing CC.

ifeq (default,$(origin CC))
    ifeq ($(_IS_WINDOWS),)
        # On Linux, we only set CC to gcc
        CC=gcc
    else
        # On Windows, we set CC to the MinGW installation
        CC=C:\MinGW\bin\gcc.exe
        CXX=C:\MinGW\bin\g++.exe
        # We also must add these libraries to link, as they are referenced from
        # the TSFFS static library but cannot be statically linked into it. Because
        # the link step is controlled by SIMICS' build system, we just pass these in
        # the normal way it expects.
        #
        # NOTE: If more dependencies are implicitly added in the future, the library
        #       needed to link can be found by searching the windows-rs repository on
        #       GitHub for the function name and looking at the filename that defines
        #       the wrapper, e.g.:
        #       https://github.com/microsoft/windows-rs/blob/1a2a9920df38678d1be4c6a6a6d43489a30ef4e9/crates/targets/baseline/ntdll.dll.c#L333
        #       requires linking -lntdll
        LDFLAGS=-lws2_32 -loleaut32 -lole32 -lbcrypt -luserenv -lntdll
    endif
endif
