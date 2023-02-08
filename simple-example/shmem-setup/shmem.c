// #include "dbg.h"
//start of code
#include <sys/mman.h>
#include <sys/stat.h>        /* For mode constants */
#include <fcntl.h>           /* For O_* constants */
#include <stdio.h>

int main() {
    int fd = shm_open("dummy_afl_shm", O_CREAT | O_RDWR | O_EXCL, 0666);
    if (fd < 0) {
        fprintf(stderr, "Could not create shared mem 'dummy_afl_shm'\n");
        return fd;
    }


    unsigned int size = 16*1024*1024;
    int rv = ftruncate(fd, size);
    if (rv < 0) {
        // ERR_OUT_A("Could not truncate shared mem 'dummy_afl_shm' to size %lld", size);
        shm_unlink("dummy_afl_shm");
        return -1;
    }

    return 0;
}
//end of code
