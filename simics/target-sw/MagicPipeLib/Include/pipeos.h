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

#ifndef PIPEOS_H
#define PIPEOS_H

#ifdef _WIN32
 #include <windows.h>
 #define MP_EINVAL      ERROR_INVALID_PARAMETER /* 87 */
 #ifdef MEM_LARGE_PAGES
  #define PIPEOS_HUGEPAGE MEM_LARGE_PAGES
 #else /* !MEM_LARGE_PAGES */
  #define PIPEOS_HUGEPAGE 0x20000000
 #endif
#else /* !_WIN32 */
 #include <errno.h>
 #define MP_EINVAL      EINVAL          /* 22 */
 #ifdef MAP_HUGETLB
  #define PIPEOS_HUGEPAGE MAP_HUGETLB
 #else
  #define PIPEOS_HUGEPAGE 0x40000       /* create a huge page mapping */
 #endif
#endif /* _WIN32 */

#if defined(__cplusplus)
extern "C" {
#endif

int pipeos_errno(void);

const char *pipeos_strerror(int errnum);

#if defined(__cplusplus)
}
#endif

#endif /* PIPEOS_H */
