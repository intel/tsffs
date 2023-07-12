# Module Stubs

These module stubs were created by creating an empty simics project with:

```sh
project-setup /tmp/project
cd /tmp/project
./bin/project-setup --c-device confuse_module
./bin/project-setup --interface confuse
cp -a modules/* /path/to/repo/modules/
```

they should never have any real code in them, and should instead link to the
confuse module static library and call into it.