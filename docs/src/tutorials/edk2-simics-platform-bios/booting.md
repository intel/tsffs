# Booting the BIOS

After building our BIOS, we want to make sure we can boot it normally before we
add our fuzzing harness. This time, we'll add our harness to the boot flow, before any
UEFI shell, so it is prudent to make sure everything looks OK first.

Before this step, you'll need to have the TSFFS SIMICS package installed in your system
by following the [setup steps](../../setup/README.md) or by installing a prebuilt `ispm`
package. You'll also need the SIMICS base package (1000), the QSP-x86 package (2096),
and the QSP-CPU (8112) package. All three are available in the public simics release.

You can check that you have the package installed by running:

```sh
ispm packages --list-installed
```

You should see (at least, but likely more packages):

```txt
Installed Base Packages
 Package Number  Name         Version  Installed Paths
 1000            Simics-Base  6.0.169  /home/rhart/simics/simics-6.0.169

Installed Addon Packages
 Package Number  Name             Version    Installed Paths
 2096            QSP-x86          6.0.70     /home/rhart/simics/simics-qsp-x86-6.0.70
 8112            QSP-CPU          6.0.17     /home/rhart/simics/simics-qsp-cpu-6.0.17
 31337           TSFFS            6.0.1      /home/rhart/simics/simics-tsffs-6.0.1
```

in the list!

## Create the Project

We already created the `project` directory when we built our image, but we need to go
ahead and initialize it and add the packages we need with `ispm`.

```sh
ispm projects project --create 1000-latest 2096-latest 8112-latest 31337-latest \
  --ignore-existing-files
```

We won't be using any custom UEFI applications, so we can skip the boot disk we used in
other tutorials. We will, however, need to customize our boot script slightly.

In the previous tutorials, we used the QSP-x86 package provided `qsp-x86/uefi-shell`
target to boot directly to the UEFI shell without any extra steps. That target uses
a script to choose the boot device for us, but because the included BIOS is both
different from the one we're using and boots in release mode without debug output, we
need to modify it somewhat to work with our custom BIOS.

## Add SIMICS Targets

A SIMICS "target" is a YAML file which declares configuration options that are
ultimately passed to a script. It provides an easy way to configure and override options
without digging through scripts to find the right configuration options. You can read
more about targets
[here](https://intel.github.io/tsffs/simics/simics-user-guide/targets.html).

We'll create a new target in `project/targets/qsp-x86/qsp-uefi-custom.target.yml`:

```yaml
%YAML 1.2
---
description: QSP booting to EFI shell, defaults to empty disks
params:
  machine:
    system_info:
      type: str
      description: A short string describing what this system is.
      default: "QSP x86 - UEFI Shell"
    hardware:
      import: "%simics%/targets/qsp-x86/hardware.yml"
      defaults:
        name: qsp
        rtc:
          time: auto
        usb_tablet:
          create: true
        firmware:
          bios: ^machine:software:firmware:bios
          lan_bios:  
          spi_flash: ^machine:software:firmware:spi_flash
    uefi_device:
      advanced: 2
      name:
        type: str
        default: simics_uefi
        description: |
          Name of a simics-uefi device added under the top component.
      video_mode:
        type: int
        default: 5
        description: |
          Bochs GFX Mode to be set by UEFI BIOS during boot before OS handover.
    software:
      firmware:
        description: Firmware images
        advanced: 2
        bios:
          type: file
          description: BIOS file.
          default: "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"
        lan_bios:
          type: file
          required: false
          description: ROM BIOS file for the ICH10 LAN Ethernet adaptor
        spi_flash:
          type: file
          default: "%simics%/targets/qsp-x86/images/spi-flash.bin"
          description: The ICH10 SPI flash file to use.
        script_delay:
          type: int
          default: 1
          description: Script delay multiplier during UEFI boot
      
  network: 
    switch:
      import: "%simics%/targets/common/ethernet-setup.yml"
    service_node:
      import: "%simics%/targets/common/sn-setup.yml"
      defaults:
        ethernet_switch: ^network:switch:ethernet_switch:name
    
  output:
    system:
      type: str
      output: yes
      default: ^machine:hardware:output:system
script: "%script%/qsp-uefi-custom.target.yml.include"
...
```

This target is copied more or less wholesale from the `uefi-shell.target.yml` file in
your SIMICS QSP-x86 installation, but is modified to use a different default BIOS file,
a different `.include` script, and uses a different path to import the top level
`hardware.yml` script.

We also need to provide a custom `.include` script, which is (as the name may suggest)
included by the target and run on startup to configure the system. Most of this script
is also copied from the `uefi-shell.target.yml.include` script with the exception of the
final `script-branch`. This `script-branch` enters the BIOS boot menu and selects the
UEFI shell from it after waiting for a print message that indicates the boot menu is
visible.

```simics
run-script "%simics%/targets/qsp-x86/hardware.yml" namespace = machine:hardware

local $system = (params.get machine:hardware:output:system)

instantiate-components $system

# Add Simics UEFI meta-data device
if (params.get machine:uefi_device:name) {
        @name = f"{simenv.system}.{params['machine:uefi_device:name']}"
        @dev = SIM_create_object("simics-uefi", name, [])
        @getattr(conf, simenv.system).mb.nb.pci_bus.devices.append([0, 7, dev])
        @dev.video_mode = params['machine:uefi_device:video_mode']
}

## Name system
$system->system_info = (params.get machine:system_info)

## Set a time quantum that provides reasonable performance
set-time-quantum cell = $system.cell seconds = 0.0001

## Set up Ethernet
run-script "%simics%/targets/common/ethernet-setup.yml" namespace = network:switch
if (params.get network:switch:create_network) {
    local $ethernet_switch = (params.get network:switch:ethernet_switch:name)
    connect ($ethernet_switch.get-free-connector) (params.get machine:hardware:output:eth_slot)
    instantiate-components (params.get network:switch:ethernet_switch:name)
}
run-script "%simics%/targets/common/sn-setup.yml" namespace = network:service_node

local $system = (params.get machine:hardware:output:system)

local $system = (params.get machine:hardware:output:system)

script-branch {
        local $con = $system.serconsole.con
        # NOTE: We have to modify this from the included target because
        # the custom BIOS doesn't print the original message until the menu appears
        bp.console_string.wait-for $con "End Load Options Dumping"
        bp.time.wait-for seconds = 5.0
        echo "Got load options dump"
        echo "Opening EFI shell"
        $con.input -e Esc
        bp.time.wait-for seconds = 5.0

        $con.input -e Down
        $con.input -e Down
        $con.input -e Enter
        bp.time.wait-for seconds = 5.0

        foreach $i in (range 6) {
                $con.input -e Down
        }

        $con.input -e Enter
        $con.input -e Enter
}
```

Save this file as `project/targets/qsp-x86/qsp-uefi-custom.target.yml.include`.

## Test Booting The BIOS

With our files all in place, we can create a tiny SIMICS script, and save it as
`project/run.simics`:

```simics
load-target "qsp-x86/qsp-uefi-custom" namespace = qsp machine:hardware:firmware:bios = "%simics%/workspace/Build/SimicsOpenBoardPkg/BoardX58Ich10/DEBUG_GCC/FV/BOARDX58ICH10.fd"

script-branch {
    local $con = qsp.serconsole.con
    bp.console_string.wait-for $con "Shell>"
    bp.time.wait-for seconds = .5
    qsp.serconsole.con.input "help\n"
    bp.time.wait-for seconds = .5
}

run
```

Then run the script:

```sh
./simics -no-gui --no-win ./run.simics
```

Somewhere in the output you should see:

```txt
<qsp.serconsole.con>Shell> help\r\n
<qsp.serconsole.con>alias         - Displays, creates, or deletes UEFI Shell aliases.\r\n
<qsp.serconsole.con>attrib        - Displays or modifies the attributes of files or directories.\r\n
<qsp.serconsole.con>bcfg          - Manages the boot and driver options that are stored in NVRAM.\r\n
<qsp.serconsole.con>cd            - Displays or changes the current directory.\r\n
```

If you do, all is well! Notice that there is quite a bit more output due to being a
debug build of the BIOS.