#include "confuse_ll.h"
#include <time.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char** argv) {

  simics_handle simics;
  
  if (argc != 2) {
      printf("Please provide a path to a Simics project as an argument.\n");
      exit(1);
  }
  
  
  int rv = confuse_init(argv[1], "simple-example/simics-scripts/qsp-x86-uefi-app.yml", &simics);
  if (rv) {
      printf("Could not initialize Simics.");
      exit(-1);
  }
  
  struct timespec start, stop;
  printf("Loop start\n");
  clock_gettime(CLOCK_REALTIME, &start);
  for (int i = 0; i < 10; i ++) {
      //TODO: clear shared mems
      confuse_reset(simics);
      //TODO: Setup input data etc
      //      Maybe based on some predefined format that has
      //        - data for specific mem regions
      //        - data for specific registers
      //        - data that needs to be pushed into magic pipe
      confuse_run(simics);
      //TODO: Read out result data. Likely, no standard possible. Too specific for SWUT
      printf("Iteration %d\n", i);
      sleep(1);
  }
  clock_gettime(CLOCK_REALTIME, &stop);
  
  double duration = (stop.tv_sec - start.tv_sec) +
                    (stop.tv_nsec - start.tv_nsec) / 1000000000.0;
  
  printf("Total duration %lf \n", duration);
  return 0;

}

