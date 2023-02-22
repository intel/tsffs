#ifndef CONFUSE_AFL_WRAPPER_H
#define CONFUSE_AFL_WRAPPER_H

// #include "dbg.h"
// #include "confuse_ll.h"
// #include "confuse_dio.h"
#include <sys/shm.h>
#include <cstdio>
#include <iostream>
#include <memory>

/*
*
*   The code for creating a fork server and interfacing with AFL++ has been 
*   lifted from another intel project the Kernel Fuzzer for Xen Project
*   available here https://github.com/intel/kernel-fuzzer-for-xen-project
*   by Tamas Lengyal 
*
*   We will be changing the naming scheme to match the new project 
*
*/
#define MAP_SIZE      (1ull << 16)
extern unsigned char * afl_area_ptr;         // for the coverage area 
extern unsigned char *afl_input_ptr;                        // for input etc 
extern unsigned char *simics_area_ptr;

extern unsigned char * input;
extern size_t input_size;
extern size_t input_limit;
extern FILE *input_file;
extern char *input_path;




// This is the confuse_afl_init function. It takes no parameters 
// and returns an int for it's status. A 0 if everything executed properly
// otherwise some error code. 
// 
// it initializes the necessary functions and objects for AFL++ to work
int confuse_aflplusplus_init();


void confuse_afl_wait();
void confuse_afl_report(bool crash);


void confuse_afl_rewind(void);


void confuse_afl_instrument_location(unsigned long cur_loc);

void confuse_get_afl_input();

void confuse_open_simics_shm();
#endif