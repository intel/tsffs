/*
  © 2014 Intel Corporation

  This software and the related documents are Intel copyrighted materials, and
  your use of them is governed by the express license under which they were
  provided to you ("License"). Unless the License provides otherwise, you may
  not use, modify, copy, publish, distribute, disclose or transmit this software
  or the related documents without Intel's prior written permission.

  This software and the related documents are provided as is, with no express or
  implied warranties, other than those that are expressly stated in the License.
*/

#define _GNU_SOURCE
#include "pipemem.h"
#include "pipeos.h"
#include <unistd.h>
#include <string.h>

#include <Uefi.h>
#include <Library/MemoryAllocationLib.h>

size_t pipemem_page_size(void){
  return 4096;
}

void *
pipemem_alloc(size_t sz)
{
        void* tmp = AllocatePool(sz);
        return tmp;
}

void *
pipemem_realloc(void *ptr, size_t sz, size_t new_sz)
{
        void *dst = pipemem_alloc(new_sz);
        if (dst == NULL)
                return NULL;
        size_t cpy_sz = sz < new_sz ? sz : new_sz;
        memcpy(dst, ptr, cpy_sz);
        pipemem_free(ptr, sz);
        pipemem_populate(dst, new_sz, 0);
        return dst;
}

void
pipemem_populate(void *ptr, size_t siz, size_t offs)
{
        char *d = (char *)ptr;
        char *end = d + siz;
        size_t pg = pipemem_page_size();
        for (d += offs; d < end; d += pg) {
                volatile char c = *d;
                *d = c;
        }
}

void
pipemem_free(void *ptr, size_t sz)
{
        FreePool(ptr);
}

int
pipeos_errno(void)
{
        return errno;
}

const char *
pipeos_strerror(int errnum)
{
        int e = errnum < 0 ? -errnum : errnum;
        return strerror(e);
}
