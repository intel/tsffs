#ifndef __DBG_H__
#define __DBG_H__

#ifndef CONFUSE_DBG_LVL
# define CONFUSE_DBG_LVL 1
#endif

#include <stdio.h>

# define ERR_OUT_A(fmt, ...) \
    fprintf(stderr, "ERROR; %s: "  fmt "\n", __func__, __VA_ARGS__)

# define ERR_OUT(fmt) \
    fprintf(stderr, "ERROR; %s: "  fmt "\n", __func__)


#if CONFUSE_DBG_LVL > 0

# define DBG_OUT_A(level, fmt, ...) \
  if (level <= CONFUSE_DBG_LVL) \
      printf( "%s: "  fmt "\n", __func__, __VA_ARGS__)

# define DBG_OUT(level, fmt) \
  if (level <= CONFUSE_DBG_LVL) \
      printf( "%s: "  fmt "\n", __func__)


#else

# define DBG_OUT(level, msg, ...)
# define DBG_OUT_A(level, msg, ...)

#endif

#endif

