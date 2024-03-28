# Debugging

Hopefully not very often, but once in a while you may need to debug the TSFFS module.

The easiest way to do this is by loading and using it in a script that does what you
want. For example, early in development there was a bug when calling the interface
API.


So this script was used to help debug:

```txt
load-module tsffs
@tsffs = SIM_create_object(SIM_get_class("tsffs"), "tsffs", [])
tsffs.log-level 4
@import time
@print("Sleeping")
@time.sleep(30)
# Call your API here
```

ALl this script does is sleep for 30 seconds, then call the API we care about. The 30
second sleep gives you enough time to run this script, find the PID of the simics
process, and attach it with GDB. Once in GDB, just break all threads on the place you
want to debug.


```sh
$ ./simics -no-gui --no-win ./test.simics
$ ps aux | grep simics | grep -v grep | awk '{print $2}'
134284
$ gdb -q attach 134284
gdb> thread apply all break set_corpus_directory
gdb> continue
...
```

In general, most bugs will happen in FFI code, so breakpointing should be relatively
straightforward. However, in complex cases demangling may be necessary. For this,
a new version of GDB including rustfilt is suggested.