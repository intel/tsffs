/*
  © 2017 Intel Corporation
*/

/* 	 
  confuse_dio-interface.h - Skeleton code to base new interface modules on
*/
	 
/* This module is a template for defining new interface types. See the
   "Defining New Interface Types" section of the
   "Simics Model Builder User's Guide" for further documentation.

   The corresponding DML definition can be found in confuse_dio_interface.dml */

#ifndef CONFUSE_DIO_INTERFACE_H
#define CONFUSE_DIO_INTERFACE_H

#include <simics/device-api.h>
#include <simics/pywrap.h>
#include <simics/simulator-api.h> 

#ifdef __cplusplus
extern "C" {
#endif


SIM_INTERFACE(confuse_dio) {
        void (*print_configured_abnormal_exits)(conf_object_t *obj);
        void (*clear_abnormal_exits)(conf_object_t *obj);
        void (*add_abnormal_exit_bp)(conf_object_t *obj, breakpoint_id_t bp, const char* message);
        void (*add_abnormal_exit_to)(conf_object_t *obj, uint64 usecs, const char* message);
};

/* Use a #define like this whenever you need to use the name of the interface
   type; the C compiler will then catch any typos at compile-time. */
#define CONFUSE_DIO_INTERFACE "confuse_dio"

#ifdef __cplusplus
}
#endif

#endif /* ! CONFUSE_DIO_INTERFACE_H */
