# Installing In Projects

After building and installing TSFFS into the local SIMICS installation (the last step in
the [Linux](../setup/linux.md#build-tsffs) and
[Windows](../setup/windows.md#build-tsffs) documents), TSFFS will be available to add
when creating projects.

- [Installing In Projects](#installing-in-projects)
  - [In New Projects](#in-new-projects)
  - [In Existing Projects](#in-existing-projects)

## In New Projects

Projects are created using `ispm` (Intel Simics Package Manager). The command below
would create a project with packages numbered 1000 (SIMICS Base), 2096 (Quick Start
Platform [QSP] x86), 8112 (QSP CPU), and 31337 (TSFFS), each with the latest version
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