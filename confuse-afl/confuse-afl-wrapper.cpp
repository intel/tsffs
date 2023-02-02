#include "confuse-afl-wrapper.h"
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#include <cstdint>
#include <csignal>
#include <getopt.h>
#include <cstring>
#include <cstdio>
#include <cstdlib>

# define ERR_OUT_A(fmt, ...) \b
    fprintf(stderr, "ERROR; %s: "  fmt "\n", __func__, __VA_ARGS__)

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

#define SHM_ENV_VAR         "__AFL_SHM_ID"
#define SHM_FUZZ_ENV_VAR    "__AFL_SHM_FUZZ_ID"
#define FORKSRV_FD          198


/* Reporting options */
#define FS_OPT_ENABLED 0x80000001
#define FS_OPT_MAPSIZE 0x40000000
#define FS_OPT_SNAPSHOT 0x20000000
#define FS_OPT_AUTODICT 0x10000000
#define FS_OPT_SHDMEM_FUZZ 0x01000000
#define FS_OPT_NEWCMPLOG 0x02000000

#define FS_OPT_MAX_MAPSIZE ((0x00fffffeU >> 1) + 1)
#define FS_OPT_SET_MAPSIZE(x) \
  (x <= 1 || x > FS_OPT_MAX_MAPSIZE ? 0 : ((x - 1) << 1))

bool afl;
unsigned char *afl_area_ptr;
unsigned char *afl_input_ptr;
static char *id_str;
static char *fuzz_str;
unsigned long prev_loc;

unsigned char * input;
size_t input_size;
FILE *input_file;
char *input_path;


int confuse_aflplusplus_init()
{
    uint32_t status = FS_OPT_ENABLED | FS_OPT_MAPSIZE | FS_OPT_SET_MAPSIZE(MAP_SIZE);
    unsigned char tmp[4];

    id_str = getenv(SHM_ENV_VAR);
    if ( !id_str )
        return -1;



    int shm_id = atoi(id_str);

    afl_area_ptr = static_cast<unsigned char *>(shmat(shm_id, NULL, 0));

    if (afl_area_ptr == (void*)-1) exit(1);
    // open shared memory and mmap the relevant area


    /* Get input via shared memory instead of file i/o */
    fuzz_str = getenv(SHM_FUZZ_ENV_VAR);
    if ( fuzz_str )
    {
        int shm_fuzz_id = atoi(fuzz_str);
        afl_input_ptr = static_cast<unsigned char *>(shmat(shm_fuzz_id, NULL, 0));

        if (afl_input_ptr == (void*)-1) exit(1);

        status |= FS_OPT_SHDMEM_FUZZ;
    }

    memcpy(tmp, &status, 4);

    /* Tell AFL we are alive */
    if (write(FORKSRV_FD + 1, tmp, 4) == 4)
    {
        afl = true;
    }

    return 0;
}


/*
 * Let's wait for AFL to send us something down the pipe
 * and respond with a fake pid as if the forkserver was running.
 * We do this because we don't actually need to fork the process,
 * we have already forked the VM, so this is just to keep AFL happy.
 */
void confuse_afl_wait(void)
{
    unsigned char tmp[4];
    /* Whoops, parent dead? */
    if (read(FORKSRV_FD, tmp, 4) != 4)
    {
        afl = false;
        return;
    }

    pid_t pid = getpid();
    if (write(FORKSRV_FD + 1, &pid, 4) != 4)
        afl = false;
}

/* Send AFL the crash report */
void confuse_afl_report(bool crash)
{
    int32_t status = crash ? SIGABRT : 0;
    if (write(FORKSRV_FD + 1, &status, 4) != 4)
        afl = false;
}


void confuse_get_afl_input()
{

    input_size = *(uint32_t*)afl_input_ptr;
    input = afl_input_ptr + 4;

}