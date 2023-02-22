#ifndef __CONFUSE_LL_H__
#define __CONFUSE_LL_H__

#ifdef __cplusplus
extern "C" {
#endif 

#include <sys/types.h>
typedef pid_t simics_handle;

int confuse_init(const char* simics_prj, const char* config, simics_handle* simics);
int confuse_reset(const simics_handle simics);
int confuse_run(const simics_handle simics);


#ifdef __cplusplus
}
#endif 
#endif
