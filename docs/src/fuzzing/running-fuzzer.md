# Running the Fuzzer

Once a fuzzing campaign is set up, you can generally run the fuzzer like:

```sh
./simics -no-gui --no-win --batch-mode fuzz.simics
```

At a log level (`tsffs.log-level 2`) of `2` or greater, you'll see statistics of the
current progress during execution.