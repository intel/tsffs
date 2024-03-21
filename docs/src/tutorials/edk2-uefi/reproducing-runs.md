# Reproducing Runs

It is unlikely you'll find any bugs with this harness (if you do, report them to edk2!),
but we can still test the "repro" functionality which allows you to replay an execution
of a testcase from an input file. After pressing Ctrl+C during execution, list the
corpus files (tip: `!` in front of a line in the SIMICS console lets you run shell
commands):

```txt
simics> !ls corpus
0
1
2
3
4385dc33f608888d
5b7dc5642294ccb9
```

You will probably have several files. Let's examine testcase `4385dc33f608888d`:

```txt
simics> !hexdump -C corpus/4385dc33f608888d | head -n 2
00000000  30 82 04 e8 30 82 04 53  a0 03 02 01 02 02 1d 58  |0...0..S.......X|
00000010  74 4e e3 aa f9 7e e8 ff  2f 67 53 31 6e 62 3d 1e  |tN...~../gS1nb=.|
```

We can tell the fuzzer that we want to run with this specific input by using:

```txt
simics> @tsffs.iface.fuzz.repro("%simics%/corpus/4385dc33f608888d")
```

The simulation will run once with this input, then output a message that you can replay
the simulation by running:

```txt
simics> reverse-to start
```

From here, you can examine memory and registers (with `x`), single step execution (`si`)
and more! Check out the SIMICS documentation and explore all the deep debugging
capabilities that SIMICS offers. When you're done exploring, run `c` to continue.

You can change the testcase you are examining by choosing a different one with
`tsffs.iface.fuzz.repro`, but you cannot resume fuzzing after entering repro mode due
to inconsistencies with the simulated system clock.
