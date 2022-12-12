TARGET_NO_PREF_NO_SUFX=confuse
UNITS=confuse_ll confuse_dio

.PHONY: all clean objdir
.DEFAULT: all

ifneq ($(CROSSCOMPILE),)
	CC := $(CROSSCOMPILE)gcc
endif

OBJPRE ?=

TARGET_DIR ?= .
CFLAGS_COMMON= -Iinclude
CFLAGS += -O3 -std=gnu99 $(CFLAGS_COMMON)
DEP_CFLAGS = -MM
DEP_CC=$(CC)
OBJEXT=o

ifeq ($(OS),Windows_NT)
    $(error Windows OS currently not supported)
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S),Linux)
        LIBSFX=a
        LIBPFX=lib
        CCLDFLAGS=
        ARFLAGS=rs
	else
		$(error Platform not supported: $(UNAME_S))
	endif
endif

TARGET=lib/$(LIBPFX)$(TARGET_NO_PREF_NO_SUFX).$(LIBSFX)
OBJECTS=$(patsubst %,obj/$(OBJPRE)%.$(OBJEXT),$(UNITS))
DEPENDS=$(patsubst %,obj/$(OBJPRE)%.d,$(UNITS))

all: $(TARGET)

clean:
	$(RM) -rf obj lib

-include $(DEPENDS)

$(TARGET): $(OBJECTS)
	@echo "Creating library $@ "
	@mkdir -p lib
	$(AR) $(ARFLAGS) $(TARGET) $(OBJECTS)
#	$(CC) $(CCLDFLAGS) $(LDFLAGS_LOCAL_FIRST) -o $@ $^ $(LIBS)

obj/$(OBJPRE)%.$(OBJEXT): %.c
	@echo "Compiling $*.c "
	@mkdir -p obj
	$(CC) $(CFLAGS) $(CFLAGS_LOCAL) -c -o $@ $<

CDEPSED = sed 's!^\( *$(subst .,\.,$*.$(OBJEXT)) *:\)!$@ \1!g'
obj/%.d: %.c
	@mkdir -p obj
	@echo Creating dependencies: "$(<F) "
	$(DEP_CC) $(DEP_CFLAGS) $(CFLAGS_COMMON) $< | $(CDEPSED) > $@
