# Minimal Simics Module

This repository demonstrates:

- Writing a minimal SIMICS module in Rust
- Building a minimal UEFI firmware image and running it in SIMICS using the module

## Dependencies

You need to have Docker installed to build the UEFI module using EDK2. You can install
docker from [here](https://docs.docker.com/engine/install/). All that is required
for this module is that `docker --help` works correctly.