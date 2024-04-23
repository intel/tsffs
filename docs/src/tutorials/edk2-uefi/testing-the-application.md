# Testing the Application

Before we harness the application for fuzzing, we should test it to make sure it runs.

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

## Create a Project

The build script for our application created a `project` directory for us if it did not
exist, so we'll instantiate that directory as our project with `ispm`:

```sh
ispm projects project --create 1000-latest 2096-latest 8112-latest 31337-latest \
  --ignore-existing-files
cd project
```

## Get the Minimal Boot Disk

The TSFFS repository provides a boot disk called `minimal_boot_disk.craff` which
provides a filesystem and the *Simics Agent* to allow us to easily download our UEFI
application to the filesystem so we can run it. Copy the file
`examples/rsrc/minimal_boot_disk.craff` into your `project` directory.

## Create a Script

Our initial script will load (but not use *yet*) the TSFFS module, then configure and
start our simple x86-64 platform and run our UEFI application. In the `project`
directory, create `run.simics`:

```simics
# Load the TSFFS module (to make sure we can load it)
load-module tsffs

# Load the UEFI shell target with out boot disk
load-target "qsp-x86/uefi-shell" namespace = qsp machine:hardware:storage:disk0:image = "minimal_boot_disk.craff"

script-branch {
    # Wait for boot
    bp.time.wait-for seconds = 15
    qsp.serconsole.con.input "\n"
    bp.time.wait-for seconds = .5
    # Change to the FS0: filesystem (which is our mounted minimal_boot_disk.craff)
    qsp.serconsole.con.input "FS0:\n"
    bp.time.wait-for seconds = .5
    # Start the UEFI agent manager (the host side connection from the SIMICS agent)
    local $manager = (start-agent-manager)
    # Run the SIMICS agent to download our Tutorial.efi application into the simulated
    # filesystem
    qsp.serconsole.con.input ("SimicsAgent.efi --download " + (lookup-file "%simics%/Tutorial.efi") + "\n")
    bp.time.wait-for seconds = .5
    # Run our Tutorial.efi application
    qsp.serconsole.con.input "Tutorial.efi\n"
}

script-branch {
  # Wait until the application is done running, then quit
  bp.time.wait-for seconds = 30
  quit 0
}

# Start!
run
```

## Run the Test Script

Run the script:

```sh
./simics --no-win --batch-mode run.simics
```

The machine will boot, the UEFI application will run and dump out the contents of the
certificates, then the simulation will exit (this is because we passed `--batch-mode`).

Now that everything works, we're ready to move on to harnessing!