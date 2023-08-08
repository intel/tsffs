# Updating Bindings

Unless you are a developer of the fuzzer, you should't need to go through this
procedure. You are welcome to, however, if you beat the maintainers on the draw for new
SIMICS version releases.

To update bindings, first run `update-bindings.rs` according to the
[README](../simics-api-sys/scripts/README.md).

Next, add and/or update the version for SIMICS base (package 1000) in the same places as
update in [this
commit](https://github.com/intel-innersource/applications.security.fuzzing.confuse/commit/1eacccfda7b5be93d6a508add37a331ef3611f4e)

Then, bump the version for package 2096 as well, like in [this
commit](https://github.com/intel-innersource/applications.security.fuzzing.confuse/commit/7691d04604f45f52b38a205c627a7bbe55bd0922).