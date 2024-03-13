# Running the Fuzzer

Once a fuzzing campaign is set up, you can generally run the fuzzer like:

```sh
./simics --no-win --batch-mode fuzz.simics
```

At a log level of 2 or greater (i.e. set `tsffs.log-level 2` in your script) , you'll
see statistics of the current progress during execution.