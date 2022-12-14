/** @file
    A simple, basic, EDK II native, "hello" application to verify that
    we can build applications without LibC.

    Copyright (c) 2010 - 2011, Intel Corporation. All rights reserved.<BR>
    SPDX-License-Identifier: BSD-2-Clause-Patent
**/
#include  <Uefi.h>
#include  <Library/UefiLib.h>
#include  <Library/ShellCEntryLib.h>
#include  <Library/MemoryAllocationLib.h>
#include  "magic-instruction.h"
#include  "magic-pipe.h"
#include  <string.h>
#include  <unistd.h>

#pragma GCC diagnostic ignored "-Wunused-function"
#pragma GCC diagnostic ignored "-Wunused-variable"

#define KiB * 1024
#define MiB * 1024 * 1024
#define PIPE_MAGIC 0x42420f8f8ab14242ULL

//uncomment below if you want to have a human visible demo
//#define DEMO

#if 1

static int
init_magic_pipe(pipe_handle_t* p_pipe, buffer_handle_t* p_buf)
{
        const size_t buf_size = 64 MiB;

        int err = pipe_open(p_pipe, PIPE_MAGIC);
        if (err != 0) {
                fprintf(stderr, "Could not open magic pipe\n");
                return -1;
        }
        err = pipe_alloc_buf(*p_pipe, buf_size, p_buf);
        if (err) {
                fprintf(stderr, "Could not allocate pipe buffer\n");
                pipe_close(*p_pipe);
                return -1;
        }
        return 0;

}

static int
add_msg_to_pipe(buffer_handle_t buf, const char* msg)
{
        void* data;
        size_t buf_len = pipe_buf_left_ptr(buf, &data);
        size_t msg_len = strlen(msg);
        if (buf_len < (msg_len+1)) {
                fprintf(stderr, "Not enough room in buffer. Need %d. Got %d.\n"
                              , (unsigned int)msg_len, (unsigned int)buf_len);
                return -1;
        }
        memcpy(data,msg,msg_len+1);
        pipe_add_used(buf, msg_len+1, 0);
        return 0;
}

static int
get_msg_from_pipe(buffer_handle_t buf, char** p_msg)
{
        size_t buf_len = pipe_buf_used(buf);
        if (buf_len == 0)
            return -1;
        *p_msg = (char*)pipe_buf_data_ptr(buf);
        return strlen(*p_msg);
}
#endif

/***
  Print a welcoming message.

  Establishes the main structure of the application.

  @retval  0         The application exited normally.
  @retval  Other     An error occurred.
***/
int
main (
  IN int Argc,
  IN char **Argv
  )
{
#if 1
  buffer_handle_t buf = NULL;
  pipe_handle_t pipe;
  int err = init_magic_pipe(&pipe, &buf);
  if (err != 0){
        fprintf(stderr, "Could not init magic pipe\n");
        return -1;
  }
#endif

  MAGIC(42); //Inform Simics that we wanna have the start snapshot here

  //get test inputs
  pipe_clear_buf(buf);
  pipe_send_buf(pipe, buf); //we send nothing but get the inputs on return

  char* command=NULL;
  unsigned int len = get_msg_from_pipe(buf, &command);

  /* Enable below block if you want to see something changing in a demo or so*/
  #ifdef DEMO
  Print(L"Hello there fellow Programmer.\n");
  
  if (len)
    printf("%s\n",command);
  
  #endif
  
  if (command[0] == 'H') //force an actual crash
      __asm__ (".byte 0x06");
  
#if 1  
  int ok = (command[0] == 'A')? 0 : 1;
  pipe_clear_buf(buf);
  //report result
  if (ok)
      add_msg_to_pipe(buf, "OK");
  else
      add_msg_to_pipe(buf, "Fail");
#endif

  pipe_send_buf(pipe, buf); //send out

  return(0);
}

