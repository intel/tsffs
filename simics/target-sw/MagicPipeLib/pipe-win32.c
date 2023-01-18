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

#include "pipemem.h"
#include "pipeos.h"

/* Maybe the compiler doesn't support _Thread_local from C11 yet */
#ifdef __GNUC__
 #define thread_local __thread
#elif __STDC_VERSION__ >= 201112L
 #define thread_local _Thread_local
#elif defined(_MSC_VER)
 #define thread_local __declspec(thread)
#else
 #error Required definition of thread_local is missing
#endif

unsigned pipemem_map_flags(int op, unsigned flag)
{
        static int flags = MEM_COMMIT | MEM_RESERVE;
        if (op > 0)
                flags |= flag;
        else if (op < 0)
                flags &= ~flag;
        return flags;
}

int pipemem_map_populate(int op)
{
        return 0;
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

size_t
pipemem_page_size(void)
{
        static size_t pagesize = 0;
        if (!pagesize) {
                SYSTEM_INFO si;
                GetSystemInfo(&si);
                pagesize = si.dwPageSize;
        }
        return pagesize;
}

void *
pipemem_alloc(size_t sz)
{
        DWORD flags = pipemem_map_flags(0, 0);
        LPVOID ptr = VirtualAlloc(NULL, sz, flags, PAGE_READWRITE);

        if (ptr && !VirtualLock(ptr, sz)) {
                VirtualFree(ptr, sz, MEM_RELEASE);
                return NULL;
        }

        return ptr;
}

void *
pipemem_realloc(void *ptr, size_t sz, size_t new_sz)
{
        LPVOID dst = pipemem_alloc(new_sz);

        if (dst != NULL)
                MoveMemory(dst, ptr, sz);

        VirtualFree(ptr, sz, MEM_RELEASE);
        return dst;
}

void
pipemem_free(void *ptr, size_t sz)
{
        VirtualFree(ptr, sz, MEM_RELEASE);
}

int
pipeos_errno(void)
{
        return GetLastError();
}

const char *
pipeos_strerror(int errnum)
{
        static thread_local char buf[128];
        if (errnum < 0)
                errnum = -errnum;
        FormatMessage(FORMAT_MESSAGE_FROM_SYSTEM, 0, errnum,
                      LANG_SYSTEM_DEFAULT, buf, sizeof(buf), NULL);
        return buf;
}
