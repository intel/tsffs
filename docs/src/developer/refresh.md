# Refreshing Build Environment

In some cases, the TSFFS package environment can become desynchronized with the local
SIMICS installation. To resolve this issue, you can remove the files SIMICS/ISPM added
during setup and re-initialize the project:


```sh
rm -rf .project-properties \
    bin \
    linux64 \
    win64 \
    targets \
    .package-list \
    compiler.mk* \
    config.mk* \
    GNUmakefile \
    simics*
ispm projects $(pwd) --create --non-interactive --ignore-existing-files
bin/project-setup --force
```