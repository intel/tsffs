#include "confuse_dio.h"
#include "dbg.h"

#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>


unsigned char* confuse_create_dio_shared_mem(unsigned long long size)
{
    static char mem_name[35]; //should be okay as we have only one interface per process
    sprintf(mem_name, "/confuse-dio-shm-%016d", getpid());
    
    char fullpath[256];  
    sprintf(fullpath, "/dev/shm/%s", mem_name); //FIXME: Can that vary on Linux distros???
    remove(fullpath); //we ignore return value, if deletion really did not work, O_EXCL will fail
    int fd = shm_open(mem_name, O_CREAT | O_RDWR | O_EXCL, 0666); //we need exclusive here as we do not want to reuse
    if (fd < 0) {
        ERR_OUT_A("Could not create shared mem '%s'", mem_name);
        return NULL;
    }
    int rv = ftruncate(fd, size);
    if (fd < 0) {
        ERR_OUT_A("Could not truncate shared mem '%s' to size %lld", mem_name, size);
        shm_unlink(mem_name);
        return NULL;
    }
    return mmap(0, size, PROT_WRITE | PROT_READ, MAP_SHARED, fd, 0);
    
    /*
    NOTE: The map will go away when the process terminates, hence we do not need to worry
    about cleaning this up.
    However, the shared mem will persist. The plan is that the Simics side unlinks the shm
    as soon as it has it mmapped. This will ensure that the shm is deallocated as soon
    as both processes that have it mmapped die. So the only change for a stale (and persisting)
    shm is when the Simics side fails to start or crashes before unlinking the shm.
    So in nominal execution, shm should not leak.
    */
}
