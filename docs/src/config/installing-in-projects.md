# Installing In Projects

After building and installing TSFFS into the local SIMICS installation (the last step in
the [Linux](../setup/linux.md#build-tsffs) and
[Windows](../setup/windows.md#build-tsffs) documents), TSFFS will be available to add
when creating projects.

- [Installing In Projects](#installing-in-projects)
  - [In New Projects Using ISPM](#in-new-projects-using-ispm)
  - [In Existing Projects](#in-existing-projects)
  - [In Projects Which Do Not Use ISPM](#in-projects-which-do-not-use-ispm)

## In New Projects Using ISPM

Projects are created using `ispm` (Intel Simics Package Manager). The command below
would create a project with packages numbered 1000 (SIMICS Base), 2096 (Quick Start
Platform, or QSP, x86), 8112 (QSP CPU), and 31337 (TSFFS), each with the latest version
except SIMICS base, which here is specified as 6.0.169. All that is required to create
a new project with the TSFFS package included is to add it after the `--create` flag
to `ispm`. Using the `-latest` version is recommended for simplicity, but if you are a
TSFFS developer and need to test specific versions the version of any package may be
specified in the same way as the SIMICS base package here.

```sh
ispm projects /path/to/new-project --create 1000-6.0.169 2096-latest 8112-latest 31337-latest
```

## In Existing Projects

All SIMICS projects have a file `.package-list`, which contains a list of absolute or
relative (from the project's SIMICS base package root) paths to packages that should
be included in the project.

If all SIMICS packages are installed into an `install-dir` together, the TSFFS package
can be added by adding a line like (if your installed package version is `6.0.1`):

```txt
../simics-tsffs-6.0.1/
```

to your `.package-list` file, then running `bin/project-setup`.

If your SIMICS packages are not all installed together, the path can be absolute, like:

```txt
/absolute/path/to/installed/simics-tsffs-6.0.1/
```

You can obtain your latest installed version with:

```sh
ispm packages --list-installed --json | jq -r '[ .installedPackages[] | select(.pkgNumber == 31337) ] | ([ .[].version ] | max_by(split(".") | map(tonumber))) as $m | first(first(.[]|select(.version == $m)).paths[0])'
```

## In Projects Which Do Not Use ISPM

Some projects, including those which use custom builds of Simics, do not use the `ispm`
package manager. In these scenarios, the TSFFS package can be installed in a project by
extracting the package manually:

```sh
tar -xf simics-pkg-31337-7.0.0-linux64.ispm
tar -xf package.tar.gz
```

This will extract to a directory `simics-tsffs-7.0.0`. In your existing Simics project,
you can run:

```sh
./bin/addon-manager -s /path/to/simics-tsffs-7.0.0/
```

You should see a prompt like:

```txt
Simics 6 Add-on Package Manager
===============================

This script will configure this Simics installation to use optional
Simics add-on packages.

Default alternatives are enclosed in square brackets ([ ]).

=== Using the package list in project (/home/rhart/hub/tsffs/target/tmp/test_riscv_64_kernel_from_userspace_magic/project) ===

Configured add-on packages:
   RISC-V-CPU     7.2.0  ../simics-risc-v-cpu-7.2.0     
   RISC-V-Simple  7.1.0  ../simics-risc-v-simple-7.1.0  
   tsffs          7.0.0  ../simics-tsffs-7.0.0          

The following operations will be performed:
   -> Upgrade  tsffs  7.0.0  ../simics-tsffs-7.0.0                       
           to         7.0.0  ../../../../../packages/simics-tsffs-7.0.0  

New package list:
   RISC-V-CPU     7.2.0  ../simics-risc-v-cpu-7.2.0                  
   RISC-V-Simple  7.1.0  ../simics-risc-v-simple-7.1.0               
   tsffs          7.0.0  ../../../../../packages/simics-tsffs-7.0.0  

Do you want to update the package list? (y/n) [y]
```

Type `y` to accept each prompt.