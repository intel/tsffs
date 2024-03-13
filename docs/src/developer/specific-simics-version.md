# Targeting A Specific SIMICS Version

The Simics version the module is built against is determined by the Simics base
directory pointed to by the `SIMICS_BASE` environment variable. For example, running:

```sh
SIMICS_BASE=/home/user/simics/simics-6.0.185/ cargo simics-build
```

will build the module against Simics version 6.0.185.
