# Create a Project

Now that we have a disk image, we'll create a project
for fuzzing our Windows machine.

From the root of this repository:

```sh
cd examples/tutorials/windows-kernel
ispm projects . --create 1000-latest 2096-latest 8112-latest 1030-latest 31337-latest --ignore-existing-files
```


Make sure `windows-11.craff` is in the project
directory. Then, create a script `run.simics`. Before
we start fuzzing, we'll need to let Windows set itself
up on the new simulated hardware.

`run.simics` should look like this to initialize TSFFS and start the simulation.

```simics
$cpu_comp_class = "x86QSP2"
$disk0_image = "%simics%/windows-11.craff"
$use_vmp = FALSE
$create_usb_tablet = TRUE
$num_cores = 1
$num_threads = 2

run-command-file "%simics%/targets/qsp-x86/qsp-hdd-boot.simics"
```

