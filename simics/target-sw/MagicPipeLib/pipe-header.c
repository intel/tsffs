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

#include "pipe-header.h"
#include "compile-assert.h"

/* Protocol buffer header.

   Always page-aligned, typically 4 KiB.

   All values are defined in the simulated target endian. The simulator host is
   responsible for converting the endianness, if necessary.

   64-bit magic identifier
      Either a unique identifier, or a generic greeting magic number.

   16-bit additional memory page count
      For the purpose of this field a fixed 4 KiB page size is used, therefore
      only sizes between 4 KiB and 256 MiB are possible. The first mandatory
      page is not included in the page count.

   16-bit header data checksum
      The fletcher16 checksum is calculated for the first 14 bytes only. It is
      only meant to indicate that the buffer has a valid pipe header.
      This is necessary in order to reestablish a lost connection to a target
      agent, because it will use an unknown magic ID but the header will still
      be valid.
      This means that there is a 0.0015% chance that random data will be taken
      as a valid header, after the magic instruction, but with one additional
      constraint that the used size must be less than the size of the memory
      page count. If that is true this will be considered a new agent, unless
      the 64-bit magic identifier is known, and will not interfere with other
      communication. Note also that memory must be readable and writable for
      the given address, but the memory will be modified and result in
      memory corruption there.

   32-bit used data size
      The size does not include the buffer header, only buffer data and padding
      counted in bytes. The value is less than or equal to the size from the
      memory page count minus the buffer header. Therefore more than 28 bits
      cannot be used and the topmost 4 bits are reserved and should be zero. */
struct pipe_header {
        uint64_t magic;         /* magic identifier number */
        uint16_t pages;         /* memory page count */
        uint16_t csum;          /* fletcher16 header data checksum */
        uint32_t used;          /* used buffer size */
};
COMPILE_ASSERT(sizeof(pipe_header_t) == SIZEOF_PIPE_HEADER);

uint64_t
pipe_header_get_magic(const pipe_header_t *head)
{
        return head->magic;
}

void
pipe_header_set_magic(pipe_header_t *head, uint64_t magic)
{
        head->magic = magic;
}

size_t
pipe_header_get_size(const pipe_header_t *head)
{
        return ((size_t)head->pages + 1) << 12;
}


void
pipe_header_set_size(pipe_header_t *head, size_t size)
{
        head->pages = (uint16_t)((size >> 12) - 1);
}

size_t
pipe_header_get_used(const pipe_header_t *head)
{
        static const uint32_t mask = ((1 << 28) - 1);
        return (size_t)head->used & mask;
}

void
pipe_header_set_used(pipe_header_t *head, size_t used)
{
        static const uint32_t mask = ((1 << 28) - 1);
        head->used = (used & mask) | (head->used & ~mask);
}

bool
pipe_header_get_retry(const pipe_header_t *head)
{
        return (bool)(head->used >> 31);
}

void
pipe_header_set_retry(pipe_header_t *head, bool retry)
{
        const uint32_t bit = 1u << 31;
        if (retry)
                head->used |= bit;
        else
                head->used &= ~bit;
}

uint16_t
pipe_header_get_csum(const pipe_header_t *head)
{
        return head->csum;
}

/* Note that this function does not have any protection against overflow of the
   accumulator values, especially acu2. This limits the number of bytes that
   can be used to calculate the checksum, but does not affect the intended
   use-case. Add 1 to the data to make sure that each byte adds a value,
   because otherwise differences in zeroes are not detected. */
static uint16_t
fletcher16(const uint8_t *data, size_t bytes)
{
        uint32_t acu1 = 0xff;
        uint32_t acu2 = 0xff;

        while (bytes--) {
                acu1 += 1 + (uint32_t)*data;
                acu2 += acu1;
                data++;
        }
        /* Reduce sums to 8 bits */
        uint8_t sum1 = acu1 + (acu1 >> 8) + (acu1 >> 16) + (acu1 >> 24);
        uint8_t sum2 = acu2 + (acu2 >> 8) + (acu2 >> 16) + (acu2 >> 24);
        return ((uint16_t)sum2 << 8) + sum1;
}

static uint16_t
pipe_header_calc_csum(const pipe_header_t *head)
{
        pipe_header_t hd = *head;
        hd.csum = 0;
        return fletcher16((const uint8_t *)&hd, sizeof hd);
}

void
pipe_header_set_csum(pipe_header_t *head)
{
        head->csum = pipe_header_calc_csum(head);
}

bool
pipe_header_csum_ok(const pipe_header_t *head)
{
        if (pipe_header_get_csum(head) == pipe_header_calc_csum(head))
                return true;
        return false;
}

bool
pipe_header_size_ok(const pipe_header_t *head)
{
        if (sizeof(pipe_header_t) + pipe_header_get_used(head)
            <= pipe_header_get_size(head))
                return true;
        return false;
}

int
pipe_header_print(const pipe_header_t *head, FILE *to)
{
        return fprintf(to, "magic=0x%016llx used=%-9u pages=%-5hu csum=0x%04hx",
                       (long long unsigned int)head->magic, head->used,
                       head->pages, head->csum);
}
