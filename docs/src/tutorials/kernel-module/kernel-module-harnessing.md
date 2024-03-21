# Harnessing the Kernel Module

We will harness the kernel module two ways:

* With harness code compiled into the kernel module
* With harness code compiled into a user-space application
  that drives the kernel module

This demonstrates the flexibility of the fuzzer -- however
your real target software should be harnessed, should be
chosen.

## Kernel Module Harness

Because the build process for the buildroot is quite long (5-10 mins on a fast machine),
we will avoid compiling it twice. Modify the `device_write` function:

```c
static ssize_t device_write(struct file *file, const char __user *buffer,
                            size_t length, loff_t *offset) {
  int i;

  pr_info("device_write(%p,%p,%ld)", file, buffer, length);

  for (i = 0; i < length && i < BUF_LEN; i++) {
    get_user(message[i], buffer + i);
  }

  size_t size = BUF_LEN;
  size_t *size_ptr = &size;

  HARNESS_START(message, size_ptr);

  check(message);

  HARNESS_STOP();

  return i;
}
```

This adds our harness such that the first time the `device_write` function is called,
via a user-space application writing or using the IOCTL system call, the fuzzer will
take over and start the fuzzing loop.

## Userspace Driver Code

First, copy `tsffs.h` from the `harness` directory in the repository into
`src/tsffs.h`.

We'll also create `src/tutorial-mod-driver.c`, a user-space application which we will
use to drive the kernel module code via IOCTL.

```c
#include <fcntl.h>
#include <linux/ioctl.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <unistd.h>

#include "tsffs.h"

#define MAJOR_NUM 100
#define IOCTL_SET_MSG _IOW(MAJOR_NUM, 0, char *)
#define IOCTL_GET_MSG _IOR(MAJOR_NUM, 1, char *)
#define IOCTL_GET_NTH_BYTE _IOWR(MAJOR_NUM, 2, int)
#define DEVICE_FILE_NAME "char_dev"
#define DEVICE_PATH "/dev/char_dev"

int ioctl_set_msg(int file_desc, char *message) {
  int ret_val;

  ret_val = ioctl(file_desc, IOCTL_SET_MSG, message);

  if (ret_val < 0) {
    printf("ioctl_set_msg failed:%d\n", ret_val);
  }

  return ret_val;
}

int ioctl_get_msg(int file_desc) {
  int ret_val;
  char message[100] = {0};

  ret_val = ioctl(file_desc, IOCTL_GET_MSG, message);

  if (ret_val < 0) {
    printf("ioctl_get_msg failed:%d\n", ret_val);
  }
  printf("get_msg message:%s", message);

  return ret_val;
}

int ioctl_get_nth_byte(int file_desc) {
  int i, c;

  printf("get_nth_byte message:");

  i = 0;
  do {
    c = ioctl(file_desc, IOCTL_GET_NTH_BYTE, i++);

    if (c < 0) {
      printf("\nioctl_get_nth_byte failed at the %d'th byte:\n", i);
      return c;
    }

    putchar(c);
  } while (c != 0);

  return 0;
}

int main(void) {
  int file_desc, ret_val;
  char *msg = "AAAAAAAA\n";

  file_desc = open(DEVICE_PATH, O_RDWR);
  if (file_desc < 0) {
    printf("Can't open device file: %s, error:%d\n", DEVICE_PATH, file_desc);
    exit(EXIT_FAILURE);
  }

  ret_val = ioctl_set_msg(file_desc, msg);
  if (ret_val) goto error;

  close(file_desc);
  return 0;
error:
  close(file_desc);
  exit(EXIT_FAILURE);
}
```

This application opens the character device of our module, sets the message, and closes
the device.

## Harnessing the Userspace Driver Code

Once again, because the build process is quite long, we'll add the user-space harness
now. Modify the `main` function:

```c
int main(void) {
  int file_desc, ret_val;
  char msg[80] = {0};

  file_desc = open(DEVICE_PATH, O_RDWR);
  if (file_desc < 0) {
    printf("Can't open device file: %s, error:%d\n", DEVICE_PATH, file_desc);
    exit(EXIT_FAILURE);
  }

  size_t msg_size = 80;
  size_t *msg_size_ptr = &msg_size;

  __arch_harness_start(MAGIC_ALT_0, msg, msg_size_ptr);

  ret_val = ioctl_set_msg(file_desc, msg);

  __arch_harness_stop(MAGIC_ALT_1);

  if (ret_val) goto error;

  close(file_desc);
  return 0;
error:
  close(file_desc);
  exit(EXIT_FAILURE);
}
```

Notice that instead of using `HARNESS_START` and `HARNESS_STOP` here, we use
`__arch_harness_start` and `stop` so that we can send a signal with a different `n`
value. This allows us to keep the compiled-in harnessing in the test kernel module,
while leaving it inactive.
