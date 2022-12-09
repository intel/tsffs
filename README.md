# Simics Fuzzing Project
Brandon Marken Ph.D.
brandon.marken@intel.com


The purpose of this project is to build a fuzzing module for Simics.

Working title of the project:
**CO**ntrolled **FU**zzing with **S**imics -- **E**nhanced

or short **CONFUSE**


## Sub-directories

- `AFLplusplus`: Git submodule referencing AFL++
- `confuse-host-if`: The host side interface of the Fuzzer-to-Simics connection. It is a C library that the Fuzzer needs to link against to use it. Inspect its own README for details.
- `simics`: Generic Simics scripts and modules used by the Fuzzer-to-Simics connection. Here, "generic" means that the scripts should be agnostic towards the used Fuzzer and SW under test and hence be reusable for different fuzzing campaigns. Inspect its own README for details.
- `simple-example`: The goal of the example is to illustrate the use of the Fuzzer-to-Simics connection. It does not use a real Fuzzer. The example recreates a typical fuzzing loop (in C) to mimick fuzzing a UEFI application. Inspect its own README for details.