# Targeting A Specific SIMICS Version

To target a specific SIMICS base version when building TSFFS, choose the version you
want to target when initializing the project. For example, during the
[Linux Setup](../setup/linux.md), instead of running:


```sh
ispm projects $(pwd) --create --ignore-existing-files --non-interactive
```

If we wanted to target `simics-6.0.163`, we could run:

```sh
ispm projects $(pwd) --create 1000-6.0.163 --ignore-existing-files --non-interactive
```

If you already initialized your TSFFS project and need to reinitialize it, see
[refreshing build environment](./refresh.md).