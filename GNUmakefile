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

# Do not edit. This file will be overwritten by the project setup script.

.PHONY: default
default: all

.SUFFIXES:

# Check for make 4.x or newer
err_message=$(warning You need make 4.1 or newer in order to build modules.) \
            $(warning See "Simics Model Builder User's Guide")		     \
            $(warning for more information.)				     \
            $(error Unsupported make version)

# Check for old unsupported version that may still be in use
ifeq ($(MAKE_HOST),)
    $(warning Your version of make is older than 4.0.)
    $(err_message)
endif

# V=1 selects verbose build output (the actual commands).
# For compatibility, we accept VERBOSE=yes as well.
ifeq ($(VERBOSE),yes)
    $(warning VERBOSE=yes is deprecated, use V=1 instead)
    V:=1
endif

ifneq ($(V),1)
    MAKEFLAGS += -s
endif

include config.mk

# Convert a path to something make can understand in include
# directives etc.
makequote = $(subst $() ,\ ,$(subst \,\\,$(1)))
# Convert a path to something bash can understand.
# We use a funny representation of the strings ( and ), because make's
# parser requires parentheses to be balanced within $(...)
_shellquote = $(subst $(firstword ( )),"$(firstword ( ))",$(subst $(lastword ( )),"$(lastword ( ))",$(subst ;,\;,$(subst $() ,\ ,$(subst \,\\,$(1))))))

# Function definitions are exported to module.mk.
export _MAKEQUOTE := $(value makequote)
export _SHELLQUOTE := $(value _shellquote)

SIMICS_BASE := $(call _shellquote,$(RAW_SIMICS_BASE))
SIMICS_MODEL_BUILDER := $(SIMICS_BASE)
_M_DODOC_PKG:=$(call makequote,$(RAW_DODOC_PKG))
DODOC_PKG:=$(call _shellquote,$(RAW_DODOC_PKG))
# For eager references in config-user.mk. config/project/config.mk
# will redefine the variable to something else, which will be used by
# lazy references in config-user.mk. Lazy references will not work if
# the project path contains spaces.
SIMICS_WORKSPACE := .
SIMICS_PROJECT := .
PYTHON = $(SIMICS_BASE)/bin/mini-python
PYTHON3 = $(SIMICS_BASE)/bin/mini-python
PY2TO3 = $(SIMICS_BASE)/bin/py-2to3

_SYSTEMC_DML_PACKAGE:=$(call _shellquote,$(RAW_SYSTEMC_DML_PACKAGE))
export _SYSTEMC_DML_PACKAGE

_M_SIMICS_BASE := $(call makequote,$(RAW_SIMICS_BASE))

DMLC_DIR ?= $(SIMICS_BASE)/$(HOST_TYPE)/bin
DMLC ?= $(PYTHON) $(DMLC_DIR)/dml/python

# Put user definitions in config-user.mk
-include config-user.mk

include compiler.mk

include $(_M_SIMICS_BASE)/config/project/config.mk

ifeq ($(ENVCHECK),disable)
    ENVCHECK_FLAG=
else
    include $(_M_SIMICS_BASE)/config/project/envcheck.mk
    ENVCHECK_FLAG=$(HOST_TYPE)/.environment-check/all
endif

_TEST_RUNNER := bin/test-runner

_rm = rm -f $(1)
_rm_r = rm -rf $(1)

include $(_M_SIMICS_BASE)/config/project/toplevel-rules.mk
