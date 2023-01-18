#!/bin/bash

if [ -z "$1" ]; then
  echo "Please provide the path to the Simics projects"
  exit 1
fi

HERE=$PWD
cd $1
make clobber all
cd $HERE
make clean all
echo
echo "#### in case there were not errors: #####"
echo "#### Now please run './runme $1'"

