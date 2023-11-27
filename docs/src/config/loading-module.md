# Loading & Initializing TSFFS

Before TSFFS can be used, the module must be loaded, an instance of the fuzzer must be
created and instantiated, and the fuzzer must be configured for your target.

- [Loading \& Initializing TSFFS](#loading--initializing-tsffs)
  - [Loading the Module](#loading-the-module)
  - [Initializing the Fuzzer](#initializing-the-fuzzer)
  - [Configuring the Fuzzer](#configuring-the-fuzzer)

## Loading the Module

The TSFFS module can be loaded by running (in a SIMICS script):

```simics
load-module tsffs
```

Or, in a Python script:

```python
SIM_load_module("tsffs")
```

## Initializing the Fuzzer

"The Fuzzer" is an instance of the `tsffs` class, declared in the `tsffs` module. The
`tsffs` class can only be instantiated once in a given simulation.

You can get the `tsffs` class by running (in a Python script -- this can be done in a
SIMICS script by prefixing this line with the `@` prefix):

```python
tsffs_cls = SIM_get_class("tsffs")
```

Once we have the `tsffs_cls` an instance can be created with:

```python
tsffs = SIM_create_object(tsffs_cls, "tsffs", [])
```

The fuzzer instance is now created and ready to configure and use.

## Configuring the Fuzzer

The fuzzer is configured through its singular interface, simply called
`tsffs`. This interface is used for both configuration and control of the
fuzzer.

This interface can be accessed in Python scripts like:

```python
tsffs.iface.tsffs.interface_method_name(interface_args, ...)
```

And from SIMICS scripts like using the `@` prefix like:

```simics
@tsffs.iface.tsffs.interface_method_name(interface_args, ...)
```