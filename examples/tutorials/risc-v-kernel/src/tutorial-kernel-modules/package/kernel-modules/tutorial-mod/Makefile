obj-m += $(addsuffix .o, $(notdir $(basename $(wildcard $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/kernel-modules/tutorial-mod/*.c))))

.PHONY: all clean

all:
	$(MAKE) -C '/lib/modules/$(shell uname -r)/build' M='$(PWD)' modules

clean:
	$(MAKE) -C '$(LINUX_DIR)' M='$(PWD)' clean