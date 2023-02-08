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

#ifndef PIPE_HEADER_H
#define PIPE_HEADER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>

#if defined(__cplusplus)
extern "C" {
#endif

#define SIZEOF_PIPE_HEADER 16

typedef struct pipe_header pipe_header_t;

uint16_t pipe_header_get_csum(const pipe_header_t *head);
uint64_t pipe_header_get_magic(const pipe_header_t *head);
size_t pipe_header_get_size(const pipe_header_t *head);
size_t pipe_header_get_used(const pipe_header_t *head);
bool pipe_header_get_retry(const pipe_header_t *head);
bool pipe_header_csum_ok(const pipe_header_t *head);
bool pipe_header_size_ok(const pipe_header_t *head);
int pipe_header_print(const pipe_header_t *head, FILE *to);
void pipe_header_set_magic(pipe_header_t *head, uint64_t magic);
void pipe_header_set_size(pipe_header_t *head, size_t size);
void pipe_header_set_used(pipe_header_t *head, size_t used);
void pipe_header_set_csum(pipe_header_t *head);
void pipe_header_set_retry(pipe_header_t *head, bool retry);

#if defined(__cplusplus)
}
#endif

#endif /* PIPE_HEADER_H */
