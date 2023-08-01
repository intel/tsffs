/*
  © 2017 Intel Corporation
*/

/*
  tsffs_module-interface.h - Skeleton code to base new interface modules on
*/

/* This module is a template for defining new interface types. See the
   "Defining New Interface Types" section of the
   "Simics Model Builder User's Guide" for further documentation.

   The corresponding DML definition can be found in tsffs_module_interface.dml
 */

#ifndef TSFFS_MODULE_INTERFACE_H
#define TSFFS_MODULE_INTERFACE_H

#include <simics/device-api.h>
#include <simics/pywrap.h>

#ifdef __cplusplus
extern "C" {
#endif

/* If you need to define new struct types, a definition like this will make it
   possible to allocate such structs from Python using tsffs_module_data_t().

   Before doing this, you will have to load the tsffs_module_interface
   Simics module, and import the tsffs_module_interface Python module. */

/* This defines a new interface type. Its corresponding C data type will be
   called "tsffs_module_interface_t". */
SIM_INTERFACE(tsffs_module) {
  void (*init)(conf_object_t* obj);
  void (*add_processor)(conf_object_t* obj, attr_value_t* processor);
  void (*add_fault)(conf_object_t* obj, int64 fault);
  void (*add_channels)(conf_object_t* obj, attr_value_t* tx, attr_value_t* rx);
#ifndef PYWRAP
  /* methods that cannot be exported to Python, for example as it refers
     to unknown data types, must be enclosed by "#ifndef PYWRAP" ...
     "#endif". See the "Restrictions" subsection of "Defining New
     Interface Types" mentioned above. */
#endif
};

/* Use a #define like this whenever you need to use the name of the interface
   type; the C compiler will then catch any typos at compile-time. */
#define TSFFS_MODULE_INTERFACE "tsffs_module"

#ifdef __cplusplus
}
#endif

#endif /* ! TSFFS_MODULE_INTERFACE_H */
