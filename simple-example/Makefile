TARGET_NO_PREF_NO_SUFX=runme
UNITS=testcode

.PHONY: all clean
.DEFAULT: all

OBJPRE ?=

TARGET_DIR ?= .
PATH_TO_CONFUSE=../confuse-host-if
CFLAGS_COMMON= -I$(PATH_TO_CONFUSE)/include
CFLAGS += -O3 -std=gnu99 $(CFLAGS_COMMON)
DEP_CFLAGS = -MM
DEP_CC=$(CC)
OBJEXT=o
CONFUSELIB=$(PATH_TO_CONFUSE)/lib/libconfuse.a


ifeq ($(OS),Windows_NT)
    $(error Windows OS currently not supported)
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S),Linux)
        LIBSFX=a
        LIBPFX=lib
        CCLDFLAGS=
        LIBS=-L$(PATH_TO_CONFUSE)/lib -lconfuse
        ARFLAGS=rs
	else
		$(error Platform not supported: $(UNAME_S))
	endif
endif

#TARGET=lib/$(LIBPFX)$(TARGET_NO_PREF_NO_SUFX).$(LIBSFX)
TARGET=$(TARGET_NO_PREF_NO_SUFX)
OBJECTS=$(patsubst %,obj/$(OBJPRE)%.$(OBJEXT),$(UNITS))
DEPENDS=$(patsubst %,obj/$(OBJPRE)%.d,$(UNITS))

all: $(TARGET)

$(CONFUSELIB):
	$(MAKE) -C $(PATH_TO_CONFUSE) 

clean:
	$(RM) -rf obj $(TARGET)
	$(MAKE) -C $(PATH_TO_CONFUSE) clean

-include $(DEPENDS)

$(TARGET): $(CONFUSELIB) $(OBJECTS) 
	@echo "Creating executable $@ "
	$(CC) $(CCLDFLAGS) -o $@ $^ $(LIBS)

obj/$(OBJPRE)%.$(OBJEXT): %.c
	@echo "Compiling $*.c "
	@mkdir -p obj
	$(CC) $(CFLAGS) $(CFLAGS_LOCAL) -c -o $@ $<

CDEPSED = sed 's!^\( *$(subst .,\.,$*.$(OBJEXT)) *:\)!$@ \1!g'
obj/%.d: %.c
	@mkdir -p obj
	@echo Creating dependencies: "$(<F) "
	$(DEP_CC) $(DEP_CFLAGS) $(CFLAGS_COMMON) $< | $(CDEPSED) > $@
