#include "confuse_ll.h"
#include "dbg.h"

#include <unistd.h>
#include <signal.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <sys/prctl.h>

static int sig_usr2_from_simics = 0;

static void usr2_sig_handler(int signum){
    DBG_OUT(2, "Got signal");
    sig_usr2_from_simics = 1;
}

static int init_signal_hanlders() {
    struct sigaction sa_usr;
    memset(&sa_usr, 0, sizeof(struct sigaction));
    sa_usr.sa_handler = usr2_sig_handler;
    if (sigaction(SIGUSR2, &sa_usr, NULL)) {
        ERR_OUT("Could not install handler for SIGUSR2");
        return -1;
    }
    return 0;
}

static void wait_for_simics() {
    sigset_t mask, oldmask;
    /* Set up the mask of signals to temporarily block. */
    sigemptyset (&mask);
    sigaddset (&mask, SIGUSR2);
    

    /* Wait for a signal to arrive. */
    sigprocmask (SIG_BLOCK, &mask, &oldmask);
    while (!sig_usr2_from_simics)
      sigsuspend (&oldmask);
    sigprocmask (SIG_UNBLOCK, &mask, NULL);
    sig_usr2_from_simics = 0;
}


static void write_to_conf(int fd, char* line){
    int written = write(fd, line, strlen(line));
    if (written != strlen(line)) {
        ERR_OUT("Unexpected file I/O error when writing info for Simics.");
        exit(-1);
    }
}

static void generate_info_for_simics(const char* filename) {
    //TODO: This info can also hold the name of shared mem for data I/O
    //      as well as shared mem for AFL area
    int fd = open(filename, O_CREAT | O_RDWR, S_IRUSR | S_IWUSR);
    char line[256];
    sprintf(line, "if_pid:%d\n", getpid());
    write_to_conf(fd, line);
    sprintf(line, "fuzzer_shm:%s\n", "dummy_afl_shm"); //hard coded for now. Later needs to be extracted from fuzzer somehow
    write_to_conf(fd, line);
    close(fd);
}


int confuse_init(const char* simics_prj, const char* config, simics_handle* simics) {
    DBG_OUT_A(1, "Called with %s and %s", simics_prj, config);
    char tmp[256]; //FIXME: might fail on long paths!!
    sprintf(tmp, "%s/_if_data_.tmp", simics_prj);
    //TODO: Setup shared mem and get file name of AFL area shared mem
    generate_info_for_simics(tmp);
    pid_t pid = fork();
    switch (pid) {
        case 0: //child
            if (chdir(simics_prj)) {
                ERR_OUT_A("Could not change dir to Simics project %s", simics_prj);
                exit(-1);
            }
            int rv = prctl(PR_SET_PDEATHSIG, SIGKILL); //ensure Simics dies when test dies when init caller dies
                                                       //we need to see if that makes sense when using AFL
            DBG_OUT(1, "Starting Simics.");
            //TODO: add possiblity to not start in batch-mode (for demos)
            //      Maybe we should parse the config ourselves and extract some values we want
            rv = execlp("./simics", "./simics",  config, "-batch-mode", "-e", "@SIM_main_loop()", NULL);
            //TODO: check rvs for the two calls above
            break;
        default: //parent
            if (pid < 0) {
                ERR_OUT("Could not create child process");
                return -1;
            }
            *simics = pid;
    }
    //parent only. Child will never get here
    DBG_OUT_A(1, "Child created as PID %d, I am %d", pid, getpid());
    
    int rV = init_signal_hanlders();
    if (rV) return rV;
    wait_for_simics();

    return 0;
}

int confuse_reset(const simics_handle simics){
    if (kill(simics, SIGUSR2)) {
        ERR_OUT("Could not send SIGUSR2 to Simics");
        return -1;
    }
    wait_for_simics();
    return 0;   
}


int confuse_run(const simics_handle simics) {
    //TODO: Is there a possible race here? could simics be done before we reach the wait?
    if (kill(simics, SIGUSR1)) {
        ERR_OUT("Could not send SIGUSR1 to Simics");
        return -1;
    }
    wait_for_simics();
    return 0;   
}

