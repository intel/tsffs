# Running the Fuzzer

## Generate a Corpus

Because we have inside knowledge that this is an extremely simple test, we'll generate a
corpus ourselves.

```sh
mkdir -p project/corpus/
for i in $(seq 5); do
  echo -n "$(bash -c 'echo $RANDOM')" | sha256sum | head -c 8 > "project/corpus/${i}"
done
```

## Create a Project

The build script for our application created a `project` directory for us if it did not
exist, so we'll instantiate that directory as our project with `ispm`:

```sh
ispm projects project --create 1000-latest 2096-latest 2050-latest 2053-latest 8112-latest 31337-latest \
  --ignore-existing-files
cd project
```

## Configuring the Fuzzer

Create a script `project/run.simics`. First, we'll set up the fuzzer for harnessing in
the kernel module, using the default start/stop on harness.

```simics
load-module tsffs
init-tsffs

tsffs.log-level 4
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
@tsffs.timeout = 3.0
@tsffs.exceptions = [14]

load-target "risc-v-simple/linux" namespace = riscv machine:hardware:storage:disk1:image = "test.fs.craff"

script-branch {
    bp.time.wait-for seconds = 15
    board.console.con.input "mkdir /mnt/disk0\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "mount /dev/vdb /mnt/disk0\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "insmod /mnt/disk0/tutorial-mod.ko\r\n"
    bp.time.wait-for seconds = 1.0
    board.console.con.input "/mnt/disk0/tutorial-mod-driver\r\n"
}

run
```

## Run the Test Script

Run the script:

```sh
./simics -no-gui --no-win --batch-mode run.simics
```

The machine will boot to Linux, mount the disk, and run the driver application. The
driver application will call into the kernel module, and the fuzzer will start fuzzing.

## Switch Harnesses

To change harnesses, instead harnessing via the user-space
driver program, the same target software should be used. Only the two lines:

```simics
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
```

Should be changed to:

```simics
@tsffs.start_on_harness = False
@tsffs.stop_on_harness = False
@tsffs.magic_start = 4
@tsffs.magic_stop = 5
```

You can run the script again -- this time, the fuzzing loop will instantiate in the
user-space application code, run through the transition between user-space and
kernel-space caused by the `ioctl` system call, and run until the stop code in the
user-space application. This is slower (because more code is running in the simulation
in total), but can be very helpful for drivers which are not trivial to harness. For
example, the internals of drivers such as network devices can be complicated, but there
are limited APIs which provide access to their entire external interface from
user-space.