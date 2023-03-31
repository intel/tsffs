/*
  © 2010 Intel Corporation
*/

/*
  confuse_module.c - Skeleton code to base new device modules on
*/

#include <simics/device-api.h>

#include <simics/model-iface/transaction.h>

typedef struct {
        /* Simics configuration object */
        conf_object_t obj;

        /* USER-TODO: Add user specific members here. The 'value' member
           is only an example to show how to implement an attribute */
        unsigned value;
} empty_device_t;

/* Allocate memory for the object. */
static conf_object_t *
alloc_object(conf_class_t *cls)
{
        empty_device_t *empty = MM_ZALLOC(1, empty_device_t);
        return &empty->obj;
}

/* Initialize the object before any attributes are set. */
static void *
init_object(conf_object_t *obj)
{
        /* USER-TODO: Add initialization code for new objects here */
        return obj;
}

/* Finalize the object after attributes have been set, if needed. */
static void
finalize_object(conf_object_t *obj)
{
        /* USER-TODO: Add initialization code here that has to run after the
           attribute setters but that does not communicate with other objects
           or post events */
}

/* Initialization once all objects have been finalized, if needed. */
static void
objects_finalized(conf_object_t *obj)
{
        /* USER-TODO: Add initialization code here that communicates with other
           objects or posts events */
}

/* Called during object deletion while other objects may still be accessed. */
static void
deinit_object (conf_object_t *obj)
{
        /* USER-TODO: Remove all external references that this object has set
           up to itself, for example, breakpoints and hap callbacks */
}

/* Free memory allocated for the object. */
static void
dealloc_object(conf_object_t *obj)
{
        empty_device_t *empty = (empty_device_t *)obj;
        /* USER-TODO: Free any memory allocated for the object */
        MM_FREE(empty);
}

static exception_type_t
issue(conf_object_t *obj, transaction_t *t, uint64 addr)
{
        empty_device_t *empty = (empty_device_t *)obj;

        /* USER-TODO: Handle accesses to the device here */

        if (SIM_transaction_is_read(t)) {
                SIM_LOG_INFO(2, &empty->obj, 0, "read from offset %lld", addr);
                SIM_set_transaction_value_le(t, 0);
        } else {
                SIM_LOG_INFO(2, &empty->obj, 0, "write to offset %lld", addr);
        }
        return Sim_PE_No_Exception;
}

static set_error_t
set_value_attribute(conf_object_t *obj, attr_value_t *val)
{
        empty_device_t *empty = (empty_device_t *)obj;
        empty->value = SIM_attr_integer(*val);
        return Sim_Set_Ok;
}

static attr_value_t
get_value_attribute(conf_object_t *obj)
{
        empty_device_t *empty = (empty_device_t *)obj;
        return SIM_make_attr_uint64(empty->value);
}

/* Called once when the device module is loaded into Simics. */
void
init_local(void)
{
        /* Define and register the device class. */
        const class_info_t class_info = {
                .alloc = alloc_object,
                .init = init_object,
                .finalize = finalize_object,
                .objects_finalized = objects_finalized,
                .deinit = deinit_object,
                .dealloc = dealloc_object,
                .description = "This is a long description of this class.",
                .short_desc = "single line class description",
                .kind = Sim_Class_Kind_Vanilla
        };
        /* USER-TODO: Set the name of the device class */
        conf_class_t *class = SIM_create_class("confuse_module", &class_info);

        /* Register the 'transaction' interface, which is the
           interface that is implemented by memory mapped devices. */
        static const transaction_interface_t transaction_iface = {
                .issue = issue,
        };
        SIM_REGISTER_INTERFACE(class, transaction, &transaction_iface);

        /* USER-TODO: Add any attributes for the device here */

        SIM_register_attribute(
                class, "value",
                get_value_attribute, set_value_attribute,
                Sim_Attr_Optional, "i",
                "Value containing a valid valuable valuation.");
}
