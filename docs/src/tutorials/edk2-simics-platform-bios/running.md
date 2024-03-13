# Running the Fuzzer

Now that everything is harnessed, we can run the fuzzer:

```sh
./simics --no-win fuzz.simics
```

After some time, we should be able to discover the bugs we added.