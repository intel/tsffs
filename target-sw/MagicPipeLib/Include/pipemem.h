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

#ifndef PIPEMEM_H
#define PIPEMEM_H

#include <stddef.h>

#if defined(__cplusplus)
extern "C" {
#endif

size_t pipemem_page_size(void);

void *pipemem_alloc(size_t sz);

void *pipemem_realloc(void *p, size_t sz, size_t new_sz);

void pipemem_free(void *p, size_t sz);

void pipemem_populate(void *ptr, size_t siz, size_t offs);

/* op < 0: unset flag; op = 0: get flags; op > 0: set flag */
unsigned pipemem_map_flags(int op, unsigned flag);

/* op < 0: disable; op = 0: query; op > 0: enable */
int pipemem_map_populate(int op);

#if defined(__cplusplus)
}
#endif

#endif /* PIPEMEM_H */
