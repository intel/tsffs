################################################################################
#
# test-mod
#
################################################################################

TEST_MOD_VERSION = 1.0
TEST_MOD_SITE = $(BR2_EXTERNAL_TEST_KERNEL_MODULES_PATH)/package/kernel-modules/test-mod
TEST_MOD_SITE_METHOD = local

$(eval $(kernel-module))
$(eval $(generic-package))