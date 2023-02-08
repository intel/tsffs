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

#ifndef MAGIC_PIPE_H
#define MAGIC_PIPE_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#if defined(__cplusplus)
extern "C" {
#endif

typedef struct pipe_desc *pipe_handle_t;
typedef struct buffer_desc *buffer_handle_t;

/* Open a new pipe connection.

   Arguments:
        pipe_p  the address of a pipe handle.
        magic   the initial magic number of the pipe.

   The initial magic value will be replaced by the returning value in each
   received message. The last received magic value will automatically be used
   for the next pipe_send_buf call.

   Returns 0 on success, error otherwise. */
int pipe_open(pipe_handle_t *pipe_p, uint64_t magic);

/* Close an obsolete pipe connection.

   This will free any resources held by this pipe handle.

   Arguments:
        pipe    the pipe handle.

   The pipe handle will become invalid after this call along with all buffer
   handles and pointers into any buffers. */
void pipe_close(pipe_handle_t pipe);

/* Allocate a new data buffer.

   There may be several concurrent buffers in use at any time. For instance it
   may be convenient to keep the incoming buffer while constructing the
   outgoing one.

   Arguments:
        pipe    the pipe handle.
        size    the preallocated size for buffer data.
        buf_p   the address of a buffer handle.

   At least one memory page must be allocated to fit the buffer header. More
   memory pages will be reserved as necessary to hold the messages.

   *buf_p will be updated with the new buffer handle, but left untouched in
   case of errors.

   Returns 0 on success, error otherwise. */
int pipe_alloc_buf(pipe_handle_t pipe, size_t size, buffer_handle_t *buf_p);

/* Increase the preallocated space for the buffer.

   Reallocate more space for buffer data. len is the amount of extra buffer
   space to allocate. If the buffer space limit is exceeded, an error is
   returned.

   Note that any pointers to the data in the buffer may become invalid after
   this call, as the whole buffer may have moved to another address. Relative
   offsets within the buffer will still be valid.

   Arguments:
        obuf            the outgoing buffer handle.
        len             the extra space to reallocate for buffer data.
        align_bits      the number of bits to align to, or 0.

   The align_bits argument is optionally used to add padding to achieve
   alignment to the specified number of bits. This means that more than space
   than the len argument may be allocated.

   The extra data is cleared and contains only zeroes.

   Returns 0 on success, error otherwise. */
int pipe_grow_buf(buffer_handle_t buf, size_t len, size_t align_bits);

/* Clear the buffer header, to allow recycling of the buffer.

   The buffer retains its size and magic, but any used data is cleared.

   Arguments:
        buf             the buffer handle.
*/
void pipe_clear_buf(buffer_handle_t buf);

/* Get the currently allocated buffer size.

   Arguments:
        buf     the buffer handle.

   Returns the total size of the allocated buffer space. */
size_t pipe_buf_size(buffer_handle_t buf);

/* Get the current size of the data stored in the buffer.

   This number is strictly less or equal to the allocated buffer size.

   Arguments:
        buf     the buffer handle.

   Returns the size of the data stored in the buffer. */
size_t pipe_buf_used(buffer_handle_t buf);

/* Update the used data size of the buffer header.

   This call is necessary to update the number of bytes used in the buffer, and
   will allow the next call to pipe_buf_left_ptr to return an updated pointer.

   Arguments:
        buf             the buffer handle.
        used            the additional amount of used data.
        align_bits      the number of bits to align to, or 0.

   The align_bits argument is optionally used to add padding to achieve
   alignment to the specified number of bits. This means that more than used
   space will be occupied in the buffer.

   The used size will be silently truncated to the allocated size of the
   buffer, if it exceeds that limit. */
void pipe_add_used(buffer_handle_t buf, size_t used, size_t align_bits);

/* Get a pointer to the beginning of the buffer data.

   Arguments:
        buf     the buffer handle.

   Returns the address of the buffer data in the buffer. */
void *pipe_buf_data_ptr(buffer_handle_t buf);

/* Get the size and a pointer to the unused buffer space.

   This function will return the same data pointer as pipe_buf_data_ptr, until
   pipe_add_used has been called with a non-zero used size, and updated by each
   successive call to it.

   Arguments:
        buf     the buffer handle.
        data_p  the address of a buffer unused data pointer, or NULL.

   If data_p is not NULL, *data_p will be updated with the next address after
   the last used byte in the buffer.

   Returns the remaining unused space in the buffer, or 0 on error. */
size_t pipe_buf_left_ptr(buffer_handle_t buf, void **data_p);

/* Close the outgoing buffer and send it to the host.

   This function will calculate the buffer header checksum, and then execute
   the magic instruction to trigger a magic hap event in Simics. That will
   cause the magic-pipe component to wake up, if enabled, and handle the buffer
   content and write new content in return.

   Arguments:
        pipe    the pipe handle.
        buf     the outgoing buffer handle.

   The buffer handle is transformed into an incoming buffer handle with data
   from the host. The sent content of the buffer has been overwritten by the
   host, but the size and address of the buffer remain the same. */
void pipe_send_buf(pipe_handle_t pipe, buffer_handle_t buf);

/* Free an obsolete incoming buffer.

   Once an incoming buffer has been handled and it is obsolete the user must
   call this function to free the resource held by this buffer handle.

   Arguments:
        pipe    the pipe handle.
        buf     the incoming buffer handle. */
void pipe_free_buf(pipe_handle_t pipe, buffer_handle_t ibuf);

/* Get the current magic number.

   Arguments:
        pipe    the pipe handle.

   Returns the current magic number, that will be used in the next
   pipe_send_buf call. */
uint64_t pipe_get_magic(pipe_handle_t pipe);

/* Enable/disable debug information and the file they should be printed to.

   This function is intended for in development debugging only. Use with
   caution and at your own risk. */
void pipe_set_debug(pipe_handle_t pipe, unsigned mask, FILE *to);

/* Enable or disable huge page support.

   Do not use this function unless instructed to do so. */
void pipe_set_hugepage(pipe_handle_t pipe, int enable);

/* Enable or disable manual page map population.

   Do not use this function unless instructed to do so. */
void pipe_set_populate(pipe_handle_t pipe, int enable);

#if defined(__cplusplus)
}
#endif

#endif /* MAGIC_PIPE_H */
