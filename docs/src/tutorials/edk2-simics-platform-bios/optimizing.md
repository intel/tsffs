# Optimizing the Fuzzer

Without any optimization, the fuzzer will run quite slowly for this test, due to a
variety of factors. Some of these are out of our control. For example we must restore
the snapshot, and there is not much that can be done to speed up the process of
restoring the full system state.

Some, however, are under our control. Let's optimize!

## Eliminate Breakpoint Waiting

In the initial iteration of the fuzzing script, we set a breakpoint on the `DebugAssert`
function using `bp.source_location.wait-for` in a loop, and triggered a solution
manaully:

```simics
script-branch {
    while 1 {
        bp.source_location.wait-for DebugAssert -x -error-not-planted
        echo "Got breakpoint"
        @tsffs.iface.fuzz.solution(1, "DebugAssert")
    }
}
```

While this works, it's far from optimial. Not only is it slower, but we can make the
code much simpler by using `TSFFS`'s built-in breakpoint handling. Instead, we can use
the [Debugger
API](https://intel.github.io/tsffs/simics/analyzer-user-guide/debugger-api.html#examples)
to get the address of the symbol and use a traditional breakpoint, which `TSFFS` can
consume. We'll get the address of the `DebugAssert` function directly, place a
breakpoint on it using a new context, which we assign to our CPU core (this has a side
effect of using the virtual address to set the breakpoint, although in this case it is
an identity mapped address so physical addressing would work), and add that breakpoint
to the fuzzer.

```simics
load-module tsffs
init-tsffs
tsffs.log-level 4
@tsffs.start_on_harness = True
@tsffs.stop_on_harness =True
@tsffs.timeout = 3.0
@tsffs.exceptions = [13, 14]

load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -overwrite param-file = "%simics%/uefi.params" map-file = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/SimicsX58.map"
qsp.software.tracker.load-parameters "%simics%/uefi.params"
qsp.software.enable-tracker

@tcf = SIM_get_debugger()
@debug_context = tcf.iface.debug_query.matching_contexts('"UEFI Firmware"/*')[1][0]
@simenv.debug_assert_address = next(filter(lambda s: s.get("symbol") == "DebugAssert", tcf.iface.debug_symbol.list_functions(debug_context)[1])).get("address")

$ctx = (new-context)
qsp.mb.cpu0.core[0][0].set-context $ctx
$debug_assert_bp = ($ctx.break -x $debug_assert_address)
@tsffs.breakpoints = [simenv.debug_assert_bp]

run
```

This results in approximately a 2x speedup over the script branch loop.
