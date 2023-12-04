# Optimizing

There is a lot of room to optimize this test scenario. You'll notice that with full
logging on (and full hexdumping of input on), each run takes *over a second* for around
`0.3` executions per second. While this is much better than nothing, his is quite poor
performance for effective fuzzing.

## Remove Target Software Output

First, we'll `#ifdef` out our print statements in our target software:

```c
EFI_STATUS
EFIAPI
UefiMain(IN EFI_HANDLE ImageHandle, IN EFI_SYSTEM_TABLE *SystemTable) {
  UINTN MaxInputSize = 0x1000;
  UINTN InputSize = MaxInputSize;
  UINT8 *Input = (UINT8 *)AllocatePages(EFI_SIZE_TO_PAGES(MaxInputSize));

  if (!Input) {
    return EFI_OUT_OF_RESOURCES;
  }

  HARNESS_START(Input, &InputSize);

#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
  Print(L"Input: %p Size: %d\n", Input, InputSize);
#endif
  UINT8 *Cert = Input;
  UINTN CertSize = InputSize / 2;
  UINT8 *CACert = (Input + CertSize);
  UINTN CACertSize = CertSize;

#ifndef FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION
  Print(L"Certificate:\n");
  hexdump(Cert, CertSize);
  Print(L"CA Certificate:\n");
  hexdump(CACert, CACertSize);
#endif

  BOOLEAN Status = X509VerifyCert(Cert, CertSize, CACert, CACertSize);

  if (Status) {
    HARNESS_ASSERT();
  } else {
    HARNESS_STOP();
  }

  if (Input) {
    FreePages(Input, EFI_SIZE_TO_PAGES(MaxInputSize));
  }

  return EFI_SUCCESS;
}
```

```txt
[tsffs info] Stopped after 1107 iterations in 11.048448 seconds (100.19507 exec/s).
```

We are now running at 100+ iterations per second! This is a massive increase. Let's take
it a little further.

## Turn Down Logging

TSFFS logs a large amount of redundant information at high log levels (primarily for
debugging purposes). You can reduce the amount of information printed by setting:

```simics
tsffs.log-level 2
```

Where `0` is the lowest (error) and `4` is the highest (trace) logging level. Errors are
always displayed. This can typically buy a few exec/s. Note that fuzzer status messages
are printed at a logging level of `info` (2), so you likely want to at least set the
log level to 2.

This can buy us a few executions per second:

```txt
[tsffs info] [Testcase #0] run time: 0h-0m-42s, clients: 1, corpus: 21, objectives: 0, executions: 4792, exec/sec: 112.5
```

## Shorten The Testcase

In our case, we are calling one function, sandwiched between `HARNESS_START` and
`HARNESS_STOP`. There is almost nothing we can do to shorten the runtime of each
individual run here, but this is a good technique to keep in mind for your future
fuzzing efforts.

## Run More Instances

TSFFS includes stages for flushing the queue and synchronizing the queue from a shared
corpus directory. This means you can run as many instances of TSFFS as you'd like in
parallel, and they will periodically pick up new corpus entries from each other.
Execution speed scales approximately linearly across cores.

We'll launch 8 instances, all in batch mode, using `tmux`:

```sh
#!/bin/bash

SESSION_NAME="my-tsffs-campaign"

# Create a new tmux session or attach to an existing one
tmux new-session -d -s "$SESSION_NAME"

# Loop to create 8 windows and run the command in each window
for i in {1..8}; do
    # Create a new window
    tmux new-window -t "$SESSION_NAME:$i" -n "${SESSION_NAME}-window-$i"

    # Run the command in the new window
    tmux send-keys -t "$SESSION_NAME:$i" "./simics -no-gui --no-win --batch-mode run.simics" C-m
done

# Attach to the tmux session
tmux attach-session -t "$SESSION_NAME"
```

You can select each window with (for example to select window 3 `Ctrl+b 3`), and you can
detach and leave the campaign running in the background with `Ctrl+b d`. After detaching
you can reattach using the last command in the script `tmux attach-session -t
my-tsffs-campaign`. Running 8 instances of the fuzzer means approximately 8 times the
exec/s of a single instance, however each instance operates independently, so bug
finding does not scale in a correspondingly linear fashion. Regardless, the common
wisdom of more iterations being better holds.