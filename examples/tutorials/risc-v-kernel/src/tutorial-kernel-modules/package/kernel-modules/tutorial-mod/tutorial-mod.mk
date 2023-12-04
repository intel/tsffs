TUTORIAL_MOD_VERSION = 1.0
TUTORIAL_MOD_SITE = $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/kernel-modules/tutorial-mod
TUTORIAL_MOD_SITE_METHOD = local

$(eval $(kernel-module))
$(eval $(generic-package))