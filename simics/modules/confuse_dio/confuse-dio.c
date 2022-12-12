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

#include <magic_pipe_setup_interface.h>
#include <magic_pipe_reader_interface.h>
#include <magic_pipe_writer_interface.h>
#include <simics/simulator-api.h>
#include <simics/util/os.h>
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <assert.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/mman.h>
       
       


#define DIO_CLASS_NAME "confuse_dio"

/* The reserved Pipe Example magic number. */
#define PIPE_MAGIC 0x42420f8f8ab14242ULL

typedef struct {
        conf_object_t obj;

        /* The magic_pipe object to which we are connected, or NULL. */
        conf_object_t *pipe;
        const magic_pipe_setup_interface_t *pipe_su;
        const magic_pipe_reader_interface_t *pipe_rd;
        const magic_pipe_writer_interface_t *pipe_wr;

        uint64_t magic; //our magic id
        uint64_t haps;  //number of haps processed
        pid_t if_pid;
        unsigned char* shm;
        int skip_write_to_target;
        
        //char* to_target; //message to send to target
        //char* from_target; //message coming from target
        notifier_type_t ntfy_type; //our notifier type
} confuse_dio;

//get actual class from conf object prt
static confuse_dio *
confuse_dio_of_obj(conf_object_t *obj)
{
        static confuse_dio *dio = NULL;
        static conf_class_t *dio_class = NULL;
        if (dio || !obj)
                return dio;
        if (!dio_class)
                dio_class = SIM_get_class(DIO_CLASS_NAME);
        conf_class_t *obj_class = SIM_object_class(obj);
        if (obj_class == dio_class)
                dio = (confuse_dio *)obj;
        return dio;
}

/* Writer protocol callback function.
   Called when the target reads from the pipe and we are supposed to write something into it.
 */
static void
pipe_agent_writer(conf_object_t *cpu, uintptr_t bufh, uint64 magic)
{
        confuse_dio *man = confuse_dio_of_obj(NULL);
        if (man->skip_write_to_target) {
            man->skip_write_to_target = 0;
            return;
        }

        buffer_t buf = man->pipe_wr->write_data_direct(man->pipe, bufh);
        SIM_log_info(2, (conf_object_t*)man, 0, "Checking SHM");
        size_t len;
        memcpy(&len, man->shm, sizeof(size_t));
        if (len > 0) {
            SIM_log_info(3, (conf_object_t*)man, 0, "Found %ld bytes in SHM", len);
            if (buf.len < len) {
                SIM_log_error((conf_object_t*)man, 0, "Magic pipe buffer too small (%ld)!", buf.len);
                return;
            }
            memcpy(buf.data, man->shm + sizeof(size_t), len);
            SIM_log_info(3, (conf_object_t*)man, 0, "Copied %s", buf.data);
            man->pipe_wr->write_data_add(man->pipe, bufh, len);
        }
}

/* Reader protocol callback function.
   Called when the target has written into the pipe and we are supposed to take data out.
*/
static void
pipe_agent_reader(conf_object_t *cpu, uintptr_t bufh, uint64 magic)
{
        confuse_dio *man = confuse_dio_of_obj(NULL);
        man->haps++;
        size_t len = man->pipe_rd->read_buffer_size(man->pipe, bufh);
        SIM_log_info(3, (conf_object_t*)man, 0, "Getting data from pipe?");
        if (len>0) { //must be end of test
            //TODO: Figure out a well working serialization here
            SIM_log_info(2, (conf_object_t*)man, 0, "Got data from SWUT");
            bytes_t buf = man->pipe_rd->read_data_direct(man->pipe, bufh, 0);
            memcpy(man->shm,&len,sizeof(size_t));
            memcpy(man->shm + sizeof(size_t), buf.data, len);
            man->skip_write_to_target = 1;
            SIM_break_simulation(NULL);
        }
        //  NOTE: Start of test access will simply write 0 bytes
}

/* Connect to the magic pipe, by registering callbacks for some magic
   numbers. */
static set_error_t
connect_to_pipe(confuse_dio *man, conf_object_t *pipe)
{
        const magic_pipe_setup_interface_t *psu =
                SIM_c_get_interface(pipe, MAGIC_PIPE_SETUP_INTERFACE);
        const magic_pipe_reader_interface_t *prd =
                SIM_c_get_interface(pipe, MAGIC_PIPE_READER_INTERFACE);
        const magic_pipe_writer_interface_t *pwr =
                SIM_c_get_interface(pipe, MAGIC_PIPE_WRITER_INTERFACE);
        if (!psu || !prd || !pwr)
                return Sim_Set_Interface_Not_Found;

        man->pipe = pipe;
        man->pipe_su = psu;
        man->pipe_rd = prd;
        man->pipe_wr = pwr;

        /* register a reader and writer function for the reserved magic
           number. */
        psu->register_reserved_pipe(pipe, &man->obj, man->magic,
                                    pipe_agent_reader, pipe_agent_writer);
        return Sim_Set_Ok;
}

/* Disconnect from the magic pipe and close all input and output files. */
static void
disconnect_pipe(confuse_dio *man)
{
        man->pipe_su->unregister_pipe(man->pipe, &man->obj, man->magic);

        man->pipe = NULL;
        man->pipe_su = NULL;
        man->pipe_rd = NULL;
        man->pipe_wr = NULL;
}

/* Allocate a pipe manager instance. */
static conf_object_t *
dio_alloc_object(void *data)
{
        confuse_dio *man = MM_ZALLOC(1, confuse_dio);
        return &man->obj;
}

/* Initialize the pipe manager object instance. */
static void *
dio_init_object(conf_object_t *obj, void *param)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        /* Make sure the object isn't checkpointable
           because it contains external system state. */
        VT_set_object_checkpointable(obj, false);
        man->magic = PIPE_MAGIC;

        /* WARNING:
           For this simple example, we can only deal with
           messages that are 256 bytes or smaller.
        */
        //man->from_target = MM_MALLOC(256, char);
        //man->from_target[0] = 0;
        man->ntfy_type = SIM_notifier_type("MAGICPIPE_from_harness");
        return man;
}

/* Get the pipe currently used by this object. */
static attr_value_t
dio_get_pipe(void *param, conf_object_t *obj, attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        return SIM_make_attr_object(man->pipe);
}

/* Set the pipe to be used. */
static set_error_t
dio_set_pipe(void *param, conf_object_t *obj, attr_value_t *val,
              attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        if (SIM_attr_is_nil(*val)) {
                /* Attribute set to NIL; disconnect. */
                if (man->pipe)
                        disconnect_pipe(man);
                return Sim_Set_Ok;
        }

        conf_object_t *pipe = SIM_attr_object(*val);
        if (man->pipe && man->pipe != pipe) {
                /* Already connected to another pipe; disconnect first. */
                disconnect_pipe(man);
        }

        return connect_to_pipe(man, pipe);
}

/* Get the hap count attribute */
static attr_value_t
dio_get_haps(void *param, conf_object_t *obj, attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        return SIM_make_attr_uint64(man->haps);
}

/* Set the hap count attribute. Only useful for debugging purposes. */
static set_error_t
dio_set_haps(void *param, conf_object_t *obj, attr_value_t *val,
              attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        man->haps = SIM_attr_integer(*val);
        return Sim_Set_Ok;
}

/* Get the hap count attribute */
static attr_value_t
dio_get_magic(void *param, conf_object_t *obj, attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        return SIM_make_attr_uint64(man->magic);
}

/* Set the hap count attribute. Only useful for debugging purposes. */
static set_error_t
dio_set_magic(void *param, conf_object_t *obj, attr_value_t *val,
               attr_value_t *idx)
{
        confuse_dio *man = confuse_dio_of_obj(obj);
        if (man->pipe)
                return Sim_Set_Illegal_Value;
        man->magic = SIM_attr_integer(*val);
        if (!man->magic)
                man->magic = PIPE_MAGIC;
        return Sim_Set_Ok;
}

static set_error_t
dio_set_ifpid(void *param, conf_object_t *obj, attr_value_t *val,
               attr_value_t *idx)
{
        confuse_dio *dio = confuse_dio_of_obj(obj);
        dio->if_pid = (pid_t) SIM_attr_integer(*val);
        static char mem_name[35];
        sprintf(mem_name, "/confuse-dio-shm-%016d", dio->if_pid); //mem name by contract
        int fd = shm_open(mem_name, O_RDWR, 0 /*ignored anyways*/);
        if (fd < 0) {
            SIM_log_error(obj, 0, "Could not open shared mem %s", mem_name);
            return Sim_Set_Illegal_Value;
        }
        char fullpath[256];  
        sprintf(fullpath, "/dev/shm/%s", mem_name);
        struct stat st;
        stat(fullpath, &st);
        unsigned long long size = st.st_size;
        SIM_log_info(1, obj, 0, "Mapping SHM with size %lld", size);
        dio->shm = mmap(0, size, PROT_WRITE | PROT_READ, MAP_SHARED, fd, 0);
        
        //now that we have the thing mmapped, we unlink by that ensuring it
        // will be cleaned up once fuzzer and simics both die
        // underlying assumption is that fuzzer already mmapped the thing.
        shm_unlink(mem_name);

        return Sim_Set_Ok;
}

/* Register the pipe manager class and some attributes. */
void
init_local(void)
{
        class_data_t cdata = {
                .alloc_object = dio_alloc_object,
                .init_object = dio_init_object,
                .class_desc =
                "forwards data through a magic pipe connection",
                .description =
                "A class that receives and transmits data through a magic pipe"
                " connection to a pipe agent running in the target system.",
        };
        conf_class_t *cl = SIM_register_class(DIO_CLASS_NAME, &cdata);

        SIM_register_typed_attribute(cl, "haps",
                                     dio_get_haps, NULL,
                                     dio_set_haps, NULL,
                                     Sim_Attr_Optional, "i", NULL,
                                     "Magic hap count");
        SIM_register_typed_attribute(cl, "magic",
                                     dio_get_magic, NULL,
                                     dio_set_magic, NULL,
                                     Sim_Attr_Session, "i", NULL,
                                     "Magic number of the pipe agent");
        SIM_register_typed_attribute(cl, "if_pid",
                                     NULL, NULL,
                                     dio_set_ifpid, NULL,
                                     Sim_Attr_Pseudo, "i", NULL,
                                     "Inform device about interface PID");
        SIM_register_typed_attribute(cl, "pipe",
                                     dio_get_pipe, NULL,
                                     dio_set_pipe, NULL,
                                     Sim_Attr_Session, "o|n", NULL,
                                     "Connected pipe object or NIL");

}
