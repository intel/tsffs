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

#ifndef COMPILE_ASSERT_H
#define COMPILE_ASSERT_H

/* Compile time assert */
#define _COMPILE_CONCAT(x, y) x ## y
#define _COMPILE_ASSERT(row, val) typedef char \
        _COMPILE_CONCAT(compile_line_, row)[-val]
#define COMPILE_ASSERT(COND) _COMPILE_ASSERT(__LINE__, (!(COND)))

#endif /* COMPILE_ASSERT_H */
