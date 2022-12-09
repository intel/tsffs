/*
  © 2022 Intel Corporation

  This software and the related documents are Intel copyrighted materials, and
  your use of them is governed by the express license under which they were
  provided to you ("License"). Unless the License provides otherwise, you may
  not use, modify, copy, publish, distribute, disclose or transmit this software
  or the related documents without Intel's prior written permission.

  This software and the related documents are provided as is, with no express or
  implied warranties, other than those that are expressly stated in the License.
*/

/* pipe manager: a class that receives and transmits data through a magic pipe
   connection to a pipe agent running in the target system. This code, together
   with the pipe agent, examplifies how to setup and communicate over a magic
   pipe. */

#include <simics/simulator-api.h>
#include <simics/util/os.h>
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <signal.h>
//manual declaration as we do not ship include/simics/simulator/internal.h
extern void CORE_discard_future(void);
typedef void (*cb_signature)(lang_void *data);

#define CLASS_NAME "confuse_ll"


typedef struct {
        conf_object_t obj;
        pid_t if_pid;
} confuse_ll;

//get actual class from conf object prt
static confuse_ll *
confuse_ll_of_obj(conf_object_t *obj)
{
        static confuse_ll *run_ctrl = NULL;
        static conf_class_t *run_ctrl_class = NULL;
        if (run_ctrl || !obj)
                return run_ctrl;
        if (!run_ctrl_class)
                run_ctrl_class = SIM_get_class(CLASS_NAME);
        conf_class_t *obj_class = SIM_object_class(obj);
        if (obj_class == run_ctrl_class)
                run_ctrl = (confuse_ll *)obj;
        return run_ctrl;
}

static void usr1_sig_handler(int signum){
    SIM_run_alone((cb_signature)SIM_continue, 0);
}

void restore_and_clear(lang_void *data){
    VT_restore_micro_checkpoint(0);
    CORE_discard_future();
}

static void usr2_sig_handler(int signum){
    SIM_run_alone(restore_and_clear, NULL);
}


static set_error_t
trigger_usr2(void *param, conf_object_t *obj, attr_value_t *val,
               attr_value_t *idx)
{
        confuse_ll *run_ctrl = confuse_ll_of_obj(obj);
        run_ctrl->if_pid = (pid_t) SIM_attr_integer(*val);
        if (kill(run_ctrl->if_pid, SIGUSR2)) {
            SIM_log_error(obj, 0, "Could not send SIGUSR2 to pid %d", run_ctrl->if_pid);
            return Sim_Set_Illegal_Value;
        }
        return Sim_Set_Ok;
}


static void stop_callback(lang_void *callback_data, 
               conf_object_t *trigger_obj, int64 exception, 
               char *error_string)
{
    confuse_ll *run_ctrl = confuse_ll_of_obj((conf_object_t *)callback_data);
    if (kill(run_ctrl->if_pid, SIGUSR2)) {
        SIM_log_error((conf_object_t *)callback_data, 0, "Could not send SIGUSR2 to pid %d from stop handler", run_ctrl->if_pid);
    }
}

static set_error_t
arm_usr2_on_stop(void *param, conf_object_t *obj, attr_value_t *val,
               attr_value_t *idx)
{
    confuse_ll *run_ctrl = confuse_ll_of_obj(obj);
    run_ctrl->if_pid = (pid_t) SIM_attr_integer(*val);
    if (run_ctrl->if_pid) {
        SIM_log_info(1, obj, 0, "Adding hap handler");
        SIM_hap_add_callback("Core_Simulation_Stopped", stop_callback, obj);
    }
    else {
        SIM_log_info(1, obj, 0, "Removing hap handler");
        SIM_hap_delete_callback("Core_Simulation_Stopped", stop_callback, obj);
    }
    return Sim_Set_Ok;
}


//TODO: Add check for already existing object
static conf_object_t *
run_ctrl_alloc_object(void *data)
{
        confuse_ll *run_ctrl = MM_ZALLOC(1, confuse_ll);
        return &run_ctrl->obj;
}

static void *
run_ctrl_init_object(conf_object_t *obj, void *param)
{
        confuse_ll *run_ctrl = confuse_ll_of_obj(obj);
        VT_set_object_checkpointable(obj, false);
        struct sigaction sa_usr;
        memset(&sa_usr, 0, sizeof(struct sigaction));
        sa_usr.sa_handler = usr1_sig_handler;
        if (sigaction(SIGUSR1, &sa_usr, NULL)) {
            SIM_log_error(obj, 0, "Could not install handler for SIGUSR1");
        }
        sa_usr.sa_handler = usr2_sig_handler;
        if (sigaction(SIGUSR2, &sa_usr, NULL)) {
            SIM_log_error(obj, 0, "Could not install handler for SIGUSR2");
        }
        return run_ctrl;
}

void
init_local(void)
{
        class_data_t cdata = {
                .alloc_object = run_ctrl_alloc_object,
                .init_object = run_ctrl_init_object,
                .class_desc =
                "Insert SIGUSR1 and SIGUSR2 handlers for Simics",
                .description =
                "A class that install SIGUSR1 and SIGUSR2 handlers"
                "into Simics. Only one such object is allowed to exist.",
        };
        conf_class_t *cl = SIM_register_class(CLASS_NAME, &cdata);
        SIM_register_typed_attribute(cl, "send_usr2",
                                     NULL, NULL,
                                     trigger_usr2, NULL,
                                     Sim_Attr_Session, "i", NULL,
                                     "Send SIGUSR2 to a process");
        SIM_register_typed_attribute(cl, "arm_auto_send_usr2",
                                     NULL, NULL,
                                     arm_usr2_on_stop, NULL,
                                     Sim_Attr_Session, "i", NULL,
                                     "Arm auto-sending of SIGUSR2 on each sim stop");

}
