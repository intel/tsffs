#include "confuse-afl-wrapper.h"
#include <iostream>
#include <string>
#include <cstring>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/stat.h>
#include <filesystem>

static char *rand_string(char *str, size_t size)
{
    const char charset[] = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789,.-#'?!";
    if (size) {
        --size;
        for (size_t n = 0; n < size; n++) {
            int key = rand() % (int) (sizeof charset - 1);
            str[n] = charset[key];
        }
        str[size] = '\0';
    }
    return str;
}


static void print_array(unsigned char * buffer, size_t size)
{
  for(auto i=0; i==0; i++)
  {
    std::cout << buffer[i] << " ";
  }
  std::cout << std::endl;
}


int main(int argc, char** argv)
{
    simics_handle simics;

    if (argc != 2) {
        std::cout << "Please provide a path to a Simics project as an argument." << std::endl;
        exit(1);
    }

    simics_area_ptr = confuse_create_dio_shared_mem(MAP_SIZE);
    if (simics_area_ptr == NULL) {
      std::cout << "Could not allocate Simics shared memory" << std::endl;
      exit(-1);
    }

    auto rv = confuse_init(argv[1], "simics-scripts/qsp-x86-uefi-app.yml", &simics);
    if (rv) {
      std::cout << "Could not initialize Simics!" << std::endl;
      exit(-1);
    }
    // allocate the shared memory between AFL
    // this harness and simics
    auto result = confuse_aflplusplus_init();
    // confuse_open_simics_shm();

    std::cout << "Starting the fuzzing loop" << std::endl;
    for(auto i=0; i<1000; ++i)
    {
        confuse_reset(simics);

        // get input from AFL and wait
        confuse_get_afl_input();
        confuse_afl_wait();

        // write the size of hte input and the input from AFL to simics
        memcpy(simics_area_ptr, &input_size, sizeof(size));
        memcpy(simics_area_ptr+sizeof(size_t), afl_input_ptr, input_size); 
        

        // TODO currently this just reads 64 bytes 
        // we might want to find a better way to read the size of the instrumentation from simics
        
        confuse_run(simics);
        //memcpy(afl_area_ptr, simics_area_ptr, input_size);
        // read the size and the data from simics_area_ptr 
        // determine whether the run was successful, a hang, crash, etc 

        // TODO Here's where we would get the information from the branch tracer

        //here we'd tell AFL if we communicate our status to AFL 
        confuse_afl_report(false);


    }

    return 0;
}