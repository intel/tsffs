# Run the Fuzz Harness

Now we just run:

```powershell
./fuzzer.exe
```

And start testing the driver! The first thing we'll see is that it runs *really* quite
fast, which is great. If we let the fuzzer run long enough, it'll eventually decide to
generate an input that overflows the buffer, but it may take some time because we
currently have no *feedback* from the driver we're testing -- only from the fuzzer
program itself. This is "dumb fuzzing" at its finest, and we'll walk through the
various options to improve the situation, starting with the easiest.