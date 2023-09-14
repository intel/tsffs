# Scripts

These are helper scripts to manage SIMICS API low level bindings.

Example:


```sh
./scripts/update-bindings.rs -b 6.0.173 -p packages -B src/bindings/ -t ./Cargo.toml \
    -i https://af02p-or.devtools.intel.com/artifactory/simics-repos/pub/simics-installer/intel-internal/ispm-internal-latest-linux64.tar.gz \
    -s https://af02p-or.devtools.intel.com/ui/native/simics-local/pub/simics-6/linux64/
```