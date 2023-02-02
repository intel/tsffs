#include "confuse-afl-wrapper.h"
#include <iostream>


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
    std::cout << buffer[i];
  }
  std::cout << std::endl;
}


int main(int argc, char** argv)
{
    // simics_handle simics;

    // if (argc != 2) {
    //       printf("Please provide a path to a Simics project as an argument.\n");
    //       return -1;
    // }

    // // allocating MAP_SIZE memory
    // // may change to 16*1024*1024 later to match testCode
    // afl_area_ptr = confuse_create_dio_shared_mem(MAP_SIZE);

    // if(afl_area_ptr == NULL)
    // {
    //     return -1;
    // }

    // initializing simics
    // auto rv = confuse_init(argv[1], "simple-example/simics-scripts/qsp-x86-uefi-app.yml", &simics);
    // if (rv) {
    //   printf("Could not initialize Simics.\n");
    //   exit(-1);
    // }
  

    // next we'll need ot start initializing afl shared memory and connect it to simics
    auto result = confuse_aflplusplus_init();

    for(auto i=0; i<100; ++i)
    {
        confuse_get_afl_input();
        confuse_afl_wait();
        std::cout << *input << std::endl;
    }
    



    return 0;
}