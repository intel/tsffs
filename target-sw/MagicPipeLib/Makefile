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

ifneq ($(CROSSCOMPILE),)
	CC := $(CROSSCOMPILE)gcc
	AR := $(CROSSCOMPILE)ar
endif

OBJPRE ?=

ifneq ($(SIMICS_BASE),)
VPATH = $(SIMICS_BASE)/src/misc/libmagicpipe
include $(HOST_DIR)/host-config
# Ignore Simics minimum host processor dependency.
# Retain compatibility with oldest target software used in our tests.
CFLAGS_LOCAL += -march=nehalem
else
TARGET_DIR ?= .
CFLAGS_LOCAL += -std=gnu99 -fPIC
DEP_CFLAGS = -MM
DEP_CC=$(CC)
OBJEXT=o
endif

UNITS=magic-pipe pipe-header
CFLAGS_LOCAL += -static
ARFLAGS_LOCAL = rcs

ifeq ($(OS),Windows_NT)
	UNITS += pipe-win32
	CFLAGS_LOCAL += -D__USE_MINGW_ANSI_STDIO
	RM=del
else
	UNITS += pipe-unix
endif

TARGET := $(TARGET_DIR)/libmagicpipe.a
OBJECTS=$(patsubst %,$(OBJPRE)%.$(OBJEXT),$(UNITS))
DEPENDS=$(UNITS:=.d)

.PHONY: all clean
.DEFAULT: all

all: $(TARGET)

clean:
	${RM} $(TARGET) $(OBJECTS) $(DEPENDS)

-include $(DEPENDS)

$(TARGET): $(OBJECTS)
	@echo "Creating $@"
	$(AR) $(ARFLAGS_LOCAL) $@ $^

$(OBJPRE)%.$(OBJEXT): %.c
	@echo "Compiling $*.c "
	$(CC) $(CFLAGS) $(CFLAGS_LOCAL) -c -o $@ $<

CDEPSED = sed 's!^\( *$(subst .,\.,$*.$(OBJEXT)) *:\)!$@ \1!g'
%.d: %.c
	@echo Creating dependencies: "$(<F) "
	$(DEP_CC) $(DEP_CFLAGS) $(CFLAGS_COMMON) $< | $(CDEPSED) > $@
