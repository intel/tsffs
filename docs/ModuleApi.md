# Module API

The module loaded into SIMICS that the fuzzer uses to manage execution has a small
API in the form of a SIMICS module interface. That interface is defined in three places,
which are consistent with each other:

- [tsffs_module-interface.dml](tsffs_module/stubs/tsffs_module-interface/tsffs_module-interface.dml)
- [tsffs_module-interface.h](../tsffs_module/stubs/tsffs_module-interface/tsffs_module-interface.h)
- [module/mod.rs](../tsffs_module/src/module/mod.rs)

That API looks like this in Rust:

```rust
#[derive(Debug, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
/// This is the rust definition for the tffs_module_interface_t declaration in the stubs, which
/// are used to generate the interface module. This struct definition must match that one exactly
/// 
/// # Examples
/// 
/// Assuming your model is configured, and by resuming the simulation the target The
/// following SIMICS code (either in a SIMICS script, or in an equivalent Python script)
/// is typically sufficient to start the fuzzer immediately.
/// 
/// ```simics
/// stop
/// @conf.tsffs_module.iface.tsffs_module.init()
/// @conf.tsffs_module.iface.tsffs_module.add_processor(SIM_get_object(simenv.system).mb.cpu0.core[0][0])
/// # Add triple fault (special, -1 code because it has no interrupt number)
/// @conf.tsffs_module.iface.tsffs_module.add_fault(-1)
/// # Add general protection fault (interrupt #13)
/// @conf.tsffs_module.iface.tsffs_module.add_fault(13)
/// $con.input "target.efi\n"
/// # This continue is optional, the fuzzer will resume execution for you if you do not
/// continue
/// ```
pub struct ModuleInterface {
    /// Start the fuzzer. If `run` is true, this call will not return and the SIMICS main loop
    /// will be entered. If you need to run additional scripting commands after signaling the
    /// fuzzer to start, pass `False` instead, and later call either `SIM_continue()` or `run` for
    /// Python and SIMICS scripts respectively.
    pub init: extern "C" fn(obj: *mut ConfObject),
    /// Inform the module of a processor that should be traced and listened to for timeout and
    /// crash objectives. You must add exactly one processor.
    pub add_processor: extern "C" fn(obj: *mut ConfObject, processor: *mut AttrValue),
    /// Add a fault to the set of faults listened to by the fuzzer. The default set of faults is
    /// no faults, although the fuzzer frontend being used typically specifies a limited set.
    pub add_fault: extern "C" fn(obj: *mut ConfObject, fault: i64),
    /// Add channels to the module. This API should not be called by users from Python and is
    /// instead used by the fuzzer frontend to initiate communication with the module.
    pub add_channels: extern "C" fn(obj: *mut ConfObject, tx: *mut AttrValue, rx: *mut AttrValue),
}

```

The primary way of interacting with this API as a user is through Python code running
either in a Python script or a SIMICS script as your entrypoint your SIMICS project
being fuzzed uses. The documentation on the interface structure above should give you
a good idea of how to use this API. You can also take a look at the example scripts,
each of which use a slightly different pattern from each other:

- [hello-world](../examples/hello-world/rsrc/app.py)
- [x509-parse](../examples/x509-parse/rsrc/app.py)
- [mini](../examples/mini/rsrc/fuzz.simics)
- [harnessing-uefi (tutorial)](../examples/harnessing-uefi/rsrc/fuzz.simics)

## Faults

Faults are defined in [fault.rs](../tsffs_module/src/module/components/detector/fault.rs),
and fault numbers correspond to exception codes for the architecture being tested.