# Configuring

Now that we have a harnessed BIOS, we'll configure the fuzzer.

## Enabling UEFI Tracking

During fuzzing, it will be helpful to us for many reasons if we can use source-level
debugging functionality that is built into SIMICS. Recall that earlier, we made sure
that the build directory inside our Docker container is the same as the directory we
run our BIOS from. This is because we are going to use the UEFI Firmware Tracker built
into SIMICS.

We already had a `project/run.simics` script, we'll create another script
`project/fuzz.simics` which we'll build on to enable fuzzing.

We'll start with a script that just loads the platform and runs. We won't even be
booting up to the UEFI shell, only through the BIOS image load process, so we'll remove
the extra code that we had before.

```simics
load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

run
```

Next, we want to add functionality to enable UEFI tracking, which you can read about
in full detail [in the docs](https://intel.github.io/tsffs/simics/analyzer-user-guide/uefi-fw-trk.html).

At the top of the script, we'll load the tracker:


```simics
load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

run
```

Then, we need to create a new OS-awareness object (which we'll call `qsp.software`),
insert the UEFI tracker into the awareness module, and detect parameters, which we'll
save to the file "%simics%/uefi.params". This params file will contain a dictionary of
parameters like:

```python
[
    'uefi_fw_tracker',
    {
        'tracker_version': 6263,
        'map_info': [],
        'map_file': None,
        'pre_dxe_start': 0,
        'pre_dxe_size': 0,
        'dxe_start': 0,
        'dxe_size': 4294967296,
        'exec_scan_size': 327680,
        'notification_tracking': True,
        'pre_dxe_tracking': False,
        'dxe_tracking': True,
        'hand_off_tracking': True,
        'smm_tracking': True,
        'reset_tracking': True,
        'exec_tracking': True
    }
]
```

We want to enable the map file, so we'll tell the command to set the `map-file` path to
our map file. This will automatically populate the `map_info` with the info contained in
the map file. Our script will look like this:

```simics
load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -overwrite param-file = "%simics%/uefi.params" map-file = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/SimicsX58.map"
qsp.software.tracker.load-parameters "%simics%/uefi.params"
qsp.software.enable-tracker

run
```

With tracking enabled, we can add a `source_location` breakpoint on a symbol (SIMICS
will track UEFI mappings and make symbols available when they are loaded during
execution, or from a map file as we've done here). To break on assertions, we will
add a breakpoint on the `DebugAssert` function (which EDK2's `ASSERT` macro ultimately
calls).

## Configuring the Fuzzer

The above can be applied to any code which runs during the SEC, PEI, or early DXE
stages. If the codepath you want to fuzz is always executed during boot, all you need to
do is add the harness macros to it and turn on the fuzzer.

We'll use the breakpoint API to wait for the `DebugAssert` function in a loop. We do
this instead of using the `$bp_num = bp.source_location.break DebugAssert` command and
adding it to the fuzzer configuration with
`@tsffs.breakpoints = [simenv.bp_num]` because the HAP for
breakpoints does not trigger on breakpoints set on source locations in this way, so the
fuzzer cannot intercept it. This is in contrast to breakpoints set with the following,
which will work with the `tsffs` API:

```simics
$ctx = (new-context)
qsp.mb.cpu0.core[0][0].set-context $ctx
$ctx.break -w $BUFFER_ADDRESS $BUFFER_SIZE
```

The rest of the configuration is similar to configuration we've already done in previous
tutorials.

```simics
load-module tsffs
init-tsffs
tsffs.log-level 4
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
@tsffs.timeout = 3.0
@tsffs.exceptions = [13, 14]

load-module uefi-fw-tracker

load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

new-os-awareness name = qsp.software
qsp.software.insert-tracker tracker = uefi_fw_tracker_comp
qsp.software.tracker.detect-parameters -overwrite param-file = "%simics%/uefi.params" map-file = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/SimicsX58.map"
qsp.software.tracker.load-parameters "%simics%/uefi.params"
qsp.software.enable-tracker

script-branch {
    while 1 {
        bp.source_location.wait-for DebugAssert -x -error-not-planted
        echo "Got breakpoint"
        @tsffs.iface.fuzz.solution(1, "DebugAssert")
    }
}

run
```

## Obtaining a Corpus

To keep things simple, we'll go ahead and use one file as the corpus provided to us, the
actual boot image.


```sh
mkdir -p project/corpus/
curl -L -o project/corpus/0 https://raw.githubusercontent.com/tianocore/edk2-platforms/master/Platform/Intel/SimicsOpenBoardPkg/Logo/Logo.bmp
```