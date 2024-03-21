// Copyright (C) 2024 Intel Corporation
// SPDX-License-Identifier: Apache-2.0

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
  char msg[80] = {0};

  file_desc = open(DEVICE_PATH, O_RDWR);
  if (file_desc < 0) {
    printf("Can't open device file: %s, error:%d\n", DEVICE_PATH, file_desc);
    exit(EXIT_FAILURE);
  }

  size_t msg_size = 80;
  size_t *msg_size_ptr = &msg_size;

  HARNESS_START_INDEX(1, msg, msg_size_ptr);

  ret_val = ioctl_set_msg(file_desc, msg);

  HARNESS_STOP_INDEX(1);

  if (ret_val) goto error;

  close(file_desc);
  return 0;
error:
  close(file_desc);
  exit(EXIT_FAILURE);
}
