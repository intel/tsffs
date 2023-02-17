#include <fcntl.h>
#include <simics/device-api.h>
#include <simics/model-iface/cpu-instrumentation.h>
#include <simics/model-iface/processor-info.h>
#include <simics/simulator-iface/instrumentation-tool.h>
#include <simics/simulator/conf-object.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

#define MAP_SIZE_POW2 16
#define MAP_SIZE (1 << MAP_SIZE_POW2)

static void call_cb(conf_object_t *obj, conf_object_t *cpu,
                    instruction_handle_t *handle, void *user_data);

static void ret_cb(conf_object_t *obj, conf_object_t *cpu, instruction_handle_t *handle,
                   void *user_data);

static void jcc_cb(conf_object_t *obj, conf_object_t *cpu, instruction_handle_t *handle,
                   void *user_data);

typedef struct branch_info {
    const char *type;
    cpu_instruction_cb_t cb;
} branch_info_t;

branch_info_t branch_infos[] = {
    {"call", call_cb},
    {"ret", ret_cb},
    // { "jmp", jmp_cb },
    {"ja", jcc_cb},
    {"jae", jcc_cb},
    {"jb", jcc_cb},
    {"jbe", jcc_cb},
    // { "jc", jcc_cb }, == jb
    {"jcxz", jcc_cb},
    {"jecxz", jcc_cb},
    {"jrcxz", jcc_cb},
    {"je", jcc_cb},
    {"jg", jcc_cb},
    {"jge", jcc_cb},
    {"jl", jcc_cb},
    {"jle", jcc_cb},
    // { "jna", jcc_cb }, == jbe
    // { "jnae", jcc_cb }, == jb
    // { "jnb", jcc_cb }, == jae
    // { "jnbe", jcc_cb }, == ja
    // { "jnc", jcc_cb }, == jae
    {"jne", jcc_cb},
    // { "jng", jcc_cb }, == jle
    // { "jnge", jcc_cb }, == jl
    // { "jnl", jcc_cb }, == jge
    // { "jnle", jcc_cb }, == jg
    {"jno", jcc_cb},
    // { "jnp", jcc_cb }, == jpo
    {"jns", jcc_cb},
    // { "jnz", jcc_cb }, == jne
    {"jo", jcc_cb},
    {"jp", jcc_cb},
    // { "jpe", jcc_cb }, == jp
    {"jpo", jcc_cb},
    {"js", jcc_cb},
    // { "jz", jcc_cb }, == je
};

/* Cached state specific to a certain connection. */
typedef struct {
    conf_object_t obj;
    conf_object_t *cpu; /* connected cpu */

    /* Interfaces */
    const cpu_instrumentation_subscribe_interface_t *cpu_iface;
    const cpu_instruction_query_interface_t *iq_iface;
    const cpu_cached_instruction_interface_t *ci_iface;
    const processor_info_v2_interface_t *pi_iface;

    unsigned char *__p_afl_area_ptr;
    int *__interrupt_flag_ptr;
} branch_tracer_t;

FORCE_INLINE branch_tracer_t *branch_tracer_of_obj(conf_object_t *obj) {
    return (branch_tracer_t *)obj;
}

// extern unsigned char *__afl_area_ptr;

/* The equivalent of the tuple logging routine from afl-as.h. */

static inline void afl_maybe_log(branch_tracer_t *tracer, unsigned long cur_loc) {
    static __thread unsigned long prev_loc;

    // if (*(tracer->__interrupt_flag_ptr) != 0)
    //     return;

    if (tracer->__p_afl_area_ptr == NULL)
        return;

    cur_loc = (cur_loc >> 4) ^ (cur_loc << 8);
    cur_loc &= MAP_SIZE - 1;

    (tracer->__p_afl_area_ptr)[cur_loc ^ prev_loc]++;
    prev_loc = cur_loc >> 1;
}

static void call_cb(conf_object_t *obj, conf_object_t *cpu,
                    instruction_handle_t *handle, void *user_data) {
    branch_tracer_t *tracer = branch_tracer_of_obj(user_data);
    logical_address_t pc = tracer->pi_iface->get_program_counter(cpu);

    afl_maybe_log(tracer, pc);
}

static void ret_cb(conf_object_t *obj, conf_object_t *cpu, instruction_handle_t *handle,
                   void *user_data) {
    branch_tracer_t *tracer = branch_tracer_of_obj(user_data);
    logical_address_t pc = tracer->pi_iface->get_program_counter(cpu);

    afl_maybe_log(tracer, pc);
}

static void jcc_cb(conf_object_t *obj, conf_object_t *cpu, instruction_handle_t *handle,
                   void *user_data) {
    branch_tracer_t *tracer = branch_tracer_of_obj(user_data);
    logical_address_t pc = tracer->pi_iface->get_program_counter(cpu);
    afl_maybe_log(tracer, pc);
}

static void cached_instruction_cb(conf_object_t *obj, conf_object_t *cpu,
                                  cached_instruction_handle_t *ci_handle,
                                  instruction_handle_t *iq_handle, void *user_data) {
    branch_tracer_t *tracer = user_data;
    cpu_bytes_t b = tracer->iq_iface->get_instruction_bytes(cpu, iq_handle);
    attr_value_t data = SIM_make_attr_data(b.size, b.data);
    tuple_int_string_t da = tracer->pi_iface->disassemble(cpu, 0, data, 0);
    SIM_attr_free(&data);

    if (da.integer == 0) {
        printf("Cannot disassemble the instruction");
        return;
    }

    for (int i = 0; i < ALEN(branch_infos); i++) {
        strbuf_t sb = SB_INIT;
        /* compare with space in the end to distinguish between, e.g.,
           "jg" and "jge". Do not do that for "ret" which lacks space */
        sb_fmt(&sb, "%s%s", branch_infos[i].type,
               strcmp(branch_infos[i].type, "ret") == 0 ? "" : " ");
        if (strncmp(da.string, sb_str(&sb), sb_len(&sb)) == 0) {
            // if (strcmp(branch_infos[i].type, "ret")==0) SIM_log_info(1, tracer, 0,
            // "Installing on %llx", tracer->iq_iface->logical_address(cpu, iq_handle));
            tracer->ci_iface->register_instruction_after_cb(
                cpu, ci_handle, branch_infos[i].cb, user_data, NULL);
        }
        sb_free(&sb);
    }
    MM_FREE(da.string);
}

static conf_object_t *alloc_object(void *arg) {
    branch_tracer_t *pb = MM_ZALLOC(1, branch_tracer_t);
    return &pb->obj;
}

static set_error_t set_processor_attribute(conf_object_t *obj, attr_value_t *val) {
    branch_tracer_t *tracer = branch_tracer_of_obj(obj);
    conf_object_t *processor = SIM_attr_object_or_nil(*val);

    if (processor) {
        // TODO: do tracer setup. if it fails we do not have a proc and do not set the
        // attr
        tracer->cpu_iface =
            SIM_C_GET_INTERFACE(processor, cpu_instrumentation_subscribe);
        tracer->iq_iface = SIM_C_GET_INTERFACE(processor, cpu_instruction_query);
        tracer->ci_iface = SIM_C_GET_INTERFACE(processor, cpu_cached_instruction);
        tracer->pi_iface = SIM_C_GET_INTERFACE(processor, processor_info_v2);
        if ((tracer->cpu_iface == NULL) || (tracer->iq_iface == NULL) ||
            (tracer->ci_iface == NULL) || (tracer->pi_iface == NULL)) {
            SIM_LOG_ERROR(obj, 0,
                          "Provided attribute is not providing required interfaces.");
            return Sim_Set_Interface_Not_Found;
        }
        tracer->cpu_iface->register_cached_instruction_cb(
            processor, NULL, cached_instruction_cb, tracer);
    }
    tracer->cpu = processor;
    return Sim_Set_Ok;
}

static attr_value_t get_processor_attribute(conf_object_t *obj) {
    branch_tracer_t *tracer = branch_tracer_of_obj(obj);
    return SIM_make_attr_object(tracer->cpu);
}

static set_error_t set_shmem(void *param, conf_object_t *obj, attr_value_t *val,
                             attr_value_t *idx) {
    branch_tracer_t *tracer = branch_tracer_of_obj(obj);
    if (tracer->__p_afl_area_ptr) {
        SIM_log_error(obj, 0, "A shared mem was already opened before.");
        return Sim_Set_Illegal_Value;
    }
    const char *shmmem_name = SIM_attr_string(*val);
    SIM_log_info(1, obj, 0, "Opening SHM  %s", shmmem_name);
    int fd = shm_open(shmmem_name, O_RDWR, 0 /*ignored anyways*/);
    if (fd < 0) {
        SIM_log_error(obj, 0, "Could not open shared mem %s", shmmem_name);
        tracer->__p_afl_area_ptr = NULL;
        return Sim_Set_Illegal_Value;
    }
    char fullpath[256];
    sprintf(fullpath, "/dev/shm/%s", shmmem_name);
    struct stat st;
    stat(fullpath, &st);
    unsigned long long size = st.st_size;
    SIM_log_info(1, obj, 0, "Mapping SHM with size %lld", size);
    tracer->__p_afl_area_ptr = mmap(0, size, PROT_WRITE | PROT_READ, MAP_SHARED, fd, 0);
    if (tracer->__p_afl_area_ptr == NULL) {
        SIM_log_error(obj, 0, "Could not mmap shared mem %s", shmmem_name);
        close(fd);
        return Sim_Set_Illegal_Value;
    }
    return Sim_Set_Ok;
}

void init_branch_tracer_class(void) {
    static const class_data_t funcs = {.alloc_object = alloc_object,
                                       .description = "Branch tracer",
                                       .kind = Sim_Class_Kind_Session};

    conf_class_t *cl = SIM_register_class("afl_branch_tracer", &funcs);
    SIM_register_attribute(cl, "processor", get_processor_attribute,
                           set_processor_attribute, Sim_Attr_Pseudo, "o|n",
                           "The <i>processor</i> to trace.");
    SIM_register_typed_attribute(cl, "shm_name", NULL, NULL, set_shmem, NULL,
                                 Sim_Attr_Pseudo, "s", NULL,
                                 "Open provided shared mem.");
}

void init_local(void) {
    printf("INIT afl_branch_tracer\n");
    init_branch_tracer_class();
}
