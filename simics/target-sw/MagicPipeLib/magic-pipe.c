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

#include "magic-instr.h"
#define _GNU_SOURCE
#include "magic-pipe.h"
#include "pipemem.h"
#include "pipeos.h"
#include "pipe-header.h"
#include <stdlib.h>
#include <string.h>
#include <assert.h>
#include <unistd.h>
#include <inttypes.h>
#include <time.h>
#include <sys/stat.h>
#include <sys/types.h>

/* buffer descriptor */
struct buffer_desc {
        struct pipe_header *head;       /* buffer header pointer */
        struct buffer_desc *next;       /* Next in list. */
};

/* Pipe user context descriptor */
struct pipe_desc {
        struct buffer_desc *bufs;   /* List of buffers. */
        uint64_t magic;         /* pipe magic identifier */
        uint64_t count;         /* pipe event counter */
        unsigned debug;         /* debug info mask */
        FILE *odbg;             /* debug output stream */
};

static void
trigger_magic(void *ptr)
{
/* The default magic instruction hap number used by the simics agent is 12.
   WARNING! Do not changes this value unless instructed to do so. */
        MAGIC_ASM(12, ptr);
}

static void
debug_print_buffer_header(buffer_handle_t buf, FILE *to, int io)
{
        struct pipe_header *head = buf->head;
        fprintf(to, "%sHEAD@%p: ", io ? ">I>" : "<O<", head);
        pipe_header_print(head, to);
        size_t size = pipe_header_get_size(head);
        size_t used = pipe_header_get_used(head) + SIZEOF_PIPE_HEADER;
        fprintf(to, " (size=%zu KiB free=%zu)\n", size >> 10, size - used);
}

static size_t
calc_map_size(size_t len)
{
        size_t pad = pipemem_page_size() - 1;
        return (len + pad) & ~pad;
}

/* len is total buffer size, which includes all headers and data */
static int
map_proto_buf(size_t len, struct pipe_header **ph_p)
{
        if (len == 0)
                return MP_EINVAL;

        size_t map_size = calc_map_size(len);
        if (map_size > 256 * 1024 * 1024)
                return MP_EINVAL;
        assert((map_size & ((1 << 12) - 1)) == 0);

        struct pipe_header *head = pipemem_alloc(map_size);
        if (!head)
                return pipeos_errno();

        pipe_header_set_size(head, map_size);
        *ph_p = head;
        return 0;
}

/* len is the size of the whole buffer including header */
static int
new_buf_desc(size_t len, struct buffer_desc **bd_p)
{
        assert(len >= SIZEOF_PIPE_HEADER);
        struct pipe_header *head = NULL;
        int rc = map_proto_buf(len, &head);
        if (rc)
                return rc;
        assert(head);

        struct buffer_desc *bd = calloc(1, sizeof *bd);
        if (!bd) {
                pipemem_free(head, pipe_header_get_size(head));
                return pipeos_errno();
        }
        /* head buffer is returned all cleared */
        bd->head = head;
        *bd_p = bd;
        return 0;
}

static size_t
calc_aligned(size_t len, size_t align_bits)
{
        size_t pad = (1 << align_bits) - 1;
        return (len + pad) & ~pad;
}

static void
free_buf_desc(struct buffer_desc *bd)
{
        pipemem_free(bd->head, pipe_header_get_size(bd->head));
        free(bd);
}

int
pipe_open(pipe_handle_t *pipe_p, uint64_t magic)
{
        struct pipe_desc *pipe = calloc(1, sizeof(struct pipe_desc));
        if (!pipe)
                return pipeos_errno();

        pipe->magic = magic;
        pipe->odbg = stderr;
        *pipe_p = pipe;
        return 0;
}

void
pipe_close(pipe_handle_t pipe)
{
        while (pipe->bufs) {
                struct buffer_desc *next = pipe->bufs->next;
                free_buf_desc(pipe->bufs);
                pipe->bufs = next;
        }
        free(pipe);
}

int
pipe_alloc_buf(pipe_handle_t pipe, size_t size, buffer_handle_t *buf_p)
{
        if (size > 256 * 1024 * 1024 - SIZEOF_PIPE_HEADER)
                return MP_EINVAL;

        struct buffer_desc *bd = NULL;
        int rc = new_buf_desc(size + SIZEOF_PIPE_HEADER, &bd);
        if (rc)
                return rc;
        assert(bd);

        bd->next = pipe->bufs;
        pipe->bufs = bd;
        *buf_p = bd;
        return 0;
}

int
pipe_grow_buf(buffer_handle_t buf, size_t len, size_t align_bits)
{
        struct pipe_header *head = buf->head;
        size_t size = pipe_header_get_size(head);
        size_t new_size = calc_map_size(size + len);
        head = pipemem_realloc(head, size, new_size);
        if (!head)
                return pipeos_errno();

        pipe_header_set_size(head, new_size);
        buf->head = head;
        return 0;
}

void
pipe_clear_buf(buffer_handle_t buf)
{
        void *data = pipe_buf_data_ptr(buf);
        size_t len = pipe_buf_used(buf);
        if (len) {
                memset(data, 0, len);
                pipe_header_set_used(buf->head, 0);
        }
}

size_t
pipe_buf_size(buffer_handle_t buf)
{
        return pipe_header_get_size(buf->head);
}

size_t
pipe_buf_used(buffer_handle_t buf)
{
        return pipe_header_get_used(buf->head);
}

void
pipe_add_used(buffer_handle_t buf, size_t len, size_t align_bits)
{
        pipe_header_t *head = buf->head;
        size_t size = pipe_header_get_size(head);
        size_t used = pipe_header_get_used(head);
        size_t new_used = calc_aligned(used, align_bits) + len;
        assert(new_used + SIZEOF_PIPE_HEADER <= size);
        pipe_header_set_used(head, new_used);
}

void *
pipe_buf_data_ptr(buffer_handle_t buf)
{
        return (char *)buf->head + SIZEOF_PIPE_HEADER;
}

size_t
pipe_buf_left_ptr(buffer_handle_t buf, void **data_p)
{
        pipe_header_t *head = buf->head;
        size_t used = SIZEOF_PIPE_HEADER + pipe_header_get_used(head);
        if (data_p)
                *data_p = (char *)head + used;
        return pipe_header_get_size(head) - used;
}

void
pipe_send_buf(pipe_handle_t pipe, buffer_handle_t buf)
{
        struct pipe_header *head = buf->head;
        size_t used = pipe_header_get_used(head);
        assert(used + SIZEOF_PIPE_HEADER <= pipe_header_get_size(head));
        pipe_header_set_magic(head, pipe->magic);
        pipe_header_set_csum(head);
        pipe->count++;
        if (pipe->debug & 2)
                debug_print_buffer_header(buf, stderr, 0); /* out */

        trigger_magic(head);
        while (pipe_header_get_retry(head)) {
                if (pipe->debug)
                        fprintf(stderr, "Magic-pipe buffer retransmission for"
                                " hap %lld\n", pipe->count);
                pipe->count++;
                pipe_header_set_retry(head, false); /* clear bit */
                pipe_header_set_used(head, used); /* restore used size */
                pipe_header_set_csum(head);
                pipemem_populate(head, pipe_header_get_size(head), 0);
                trigger_magic(head);
        }
        pipe->magic = pipe_header_get_magic(head);
        if (pipe->debug & 1)
                debug_print_buffer_header(buf, stderr, 1); /* in */
}

void
pipe_free_buf(pipe_handle_t pipe, buffer_handle_t buf)
{
        struct buffer_desc *curr = pipe->bufs;
        if (!curr)
                return;

        struct buffer_desc *last = NULL;
        do {
                if (curr == buf) {
                        if (last)
                                last->next = curr->next;
                        else
                                pipe->bufs = curr->next;
                        free_buf_desc(buf);
                        return;
                }
                last = curr;
                curr = curr->next;
        } while (curr);
}

void *
pipe_buffer_pointer(buffer_handle_t buf)
{
        return buf->head;
}

uint64_t
pipe_get_magic(pipe_handle_t pipe)
{
        return pipe->magic;
}

void
pipe_set_debug(pipe_handle_t pipe, unsigned mask, FILE *to)
{
        pipe->debug = mask;
        pipe->odbg = to;
}

void
pipe_set_hugepage(pipe_handle_t pipe, int enable)
{
        (void)pipe;
        unsigned flag = PIPEOS_HUGEPAGE;
        pipemem_map_flags(enable ? 1 : -1, flag);
}

void
pipe_set_populate(pipe_handle_t pipe, int enable)
{
        (void)pipe;
        pipemem_map_populate(enable ? 1 : -1);
}
