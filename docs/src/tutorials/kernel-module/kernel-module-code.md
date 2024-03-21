# Kernel Module Code

Next, create `src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/Makefile`,
which will be more familiar to Linux Kernel developers (note -- you may need to convert
space indentation to tabs when pasting the contents below):

```makefile
obj-m += $(addsuffix .o, $(notdir $(basename $(wildcard $(BR2_EXTERNAL_TUTORIAL_KERNEL_MODULES_PATH)/package/kernel-modules/tutorial-mod/*.c))))

.PHONY: all clean

all:
    $(MAKE) -C '/lib/modules/$(shell uname -r)/build' M='$(PWD)' modules

clean:
    $(MAKE) -C '$(LINUX_DIR)' M='$(PWD)' clean
```

This in turn invokes the standard KBuild process, specifying our current directory
as an out of tree modules directory.

Then, copy `tsffs.h` from the `harness` directory of the repository into
`src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/tsffs.h`.

Finally, we can write our Kernel module. Doing so is well beyond the scope of this
tutorial, so copy the code below into
`src/tutorial-kernel-modules/package/kernel-modules/tutorial-mod/tutorial-mod.c`.

```c
#include <asm/errno.h>
#include <linux/atomic.h>
#include <linux/cdev.h>
#include <linux/delay.h>
#include <linux/device.h>
#include <linux/fs.h>
#include <linux/init.h>
#include <linux/ioctl.h>
#include <linux/module.h>
#include <linux/printk.h>
#include <linux/types.h>
#include <linux/uaccess.h>
#include <linux/version.h>

#include "tsffs.h"

#define MAJOR_NUM 100
#define IOCTL_SET_MSG _IOW(MAJOR_NUM, 0, char *)
#define IOCTL_GET_MSG _IOR(MAJOR_NUM, 1, char *)
#define IOCTL_GET_NTH_BYTE _IOWR(MAJOR_NUM, 2, int)
#define DEVICE_FILE_NAME "char_dev"
#define DEVICE_PATH "/dev/char_dev"
#define SUCCESS 0
#define DEVICE_NAME "char_dev"
#define BUF_LEN 80

enum {
  CDEV_NOT_USED = 0,
  CDEV_EXCLUSIVE_OPEN = 1,
};

static atomic_t already_open = ATOMIC_INIT(CDEV_NOT_USED);
static char message[BUF_LEN + 1];
static struct class *cls;

static int device_open(struct inode *inode, struct file *file) {
  pr_info("device_open(%p)\n", file);

  try_module_get(THIS_MODULE);
  return SUCCESS;
}

static int device_release(struct inode *inode, struct file *file) {
  pr_info("device_release(%p,%p)\n", inode, file);

  module_put(THIS_MODULE);
  return SUCCESS;
}
static ssize_t device_read(struct file *file, char __user *buffer,
                           size_t length, loff_t *offset) {
  int bytes_read = 0;
  const char *message_ptr = message;

  if (!*(message_ptr + *offset)) {
    *offset = 0;
    return 0;
  }

  message_ptr += *offset;

  while (length && *message_ptr) {
    put_user(*(message_ptr++), buffer++);
    length--;
    bytes_read++;
  }

  pr_info("Read %d bytes, %ld left\n", bytes_read, length);

  *offset += bytes_read;

  return bytes_read;
}

void check(char *buffer) {
  if (!strcmp(buffer, "fuzzing!")) {
    // Cause a crash
    char *x = NULL;
    *x = 0;
  }
}

static ssize_t device_write(struct file *file, const char __user *buffer,
                            size_t length, loff_t *offset) {
  int i;

  pr_info("device_write(%p,%p,%ld)", file, buffer, length);

  for (i = 0; i < length && i < BUF_LEN; i++) {
    get_user(message[i], buffer + i);
  }

  check(message);

  return i;
}

static long device_ioctl(struct file *file, unsigned int ioctl_num,
                         unsigned long ioctl_param) {
  int i;
  long ret = SUCCESS;

  if (atomic_cmpxchg(&already_open, CDEV_NOT_USED, CDEV_EXCLUSIVE_OPEN)) {
    return -EBUSY;
  }

  switch (ioctl_num) {
    case IOCTL_SET_MSG: {
      char __user *tmp = (char __user *)ioctl_param;
      char ch;

      get_user(ch, tmp);

      for (i = 0; ch && i < BUF_LEN; i++, tmp++) {
        get_user(ch, tmp);
      }

      device_write(file, (char __user *)ioctl_param, i, NULL);
      break;
    }
    case IOCTL_GET_MSG: {
      loff_t offset = 0;
      i = device_read(file, (char __user *)ioctl_param, 99, &offset);
      put_user('\0', (char __user *)ioctl_param + i);
      break;
    }
    case IOCTL_GET_NTH_BYTE:
      if (ioctl_param > BUF_LEN) {
        return -EINVAL;
      }

      ret = (long)message[ioctl_param];

      break;
  }

  atomic_set(&already_open, CDEV_NOT_USED);

  return ret;
}

static struct file_operations fops = {
    .read = device_read,
    .write = device_write,
    .unlocked_ioctl = device_ioctl,
    .open = device_open,
    .release = device_release,
};

static int __init chardev2_init(void) {
  int ret_val = register_chrdev(MAJOR_NUM, DEVICE_NAME, &fops);

  if (ret_val < 0) {
    pr_alert("%s failed with %d\n", "Sorry, registering the character device ",
             ret_val);
    return ret_val;
  }

  cls = class_create(DEVICE_FILE_NAME);
  device_create(cls, NULL, MKDEV(MAJOR_NUM, 0), NULL, DEVICE_FILE_NAME);

  pr_info("Device created on /dev/%s\n", DEVICE_FILE_NAME);

  return 0;
}

static void __exit chardev2_exit(void) {
  device_destroy(cls, MKDEV(MAJOR_NUM, 0));
  class_destroy(cls);

  unregister_chrdev(MAJOR_NUM, DEVICE_NAME);
}

module_init(chardev2_init);
module_exit(chardev2_exit);

MODULE_LICENSE("GPL");
```

To summarize, the module creates a character device which can be opened, read and
written, both via the read and write syscalls and via IOCTL. When written, the module
checks the data written against the password `fuzzing!`, and if the check passes, it
will crash itself by dereferencing NULL, which will cause a kernel panic that we will
use as a "solution" later.
