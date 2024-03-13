# Harnessing the Application

Note that as written, our application will be running the certificate verification
with uninitialized allocated memory. We want to run it instead using our fuzzer input,
so we need to add harnessing. We've already `#include`-ed our harness header file and
loaded the TSFFS module in our simulation, so we're halfway there.

## Adding Harness Code

In our `Tutorial.c` file, we'll add a few lines of code so that our main function looks
like this (the rest of the code can stay the same):

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

  Print(L"Input: %p Size: %d\n", Input, InputSize);
  UINT8 *Cert = Input;
  UINTN CertSize = InputSize / 2;
  UINT8 *CACert = (Input + CertSize);
  UINTN CACertSize = CertSize;

  Print(L"Certificate:\n");
  hexdump(Cert, CertSize);
  Print(L"CA Certificate:\n");
  hexdump(CACert, CACertSize);

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

First, we invoke `HARNESS_START` with two arguments:

* The pointer to our buffer -- this is where the fuzzer will write each testcase
* The pointer to our maximum input size (aka, the size of the buffer). The fuzzer
  records the initial value and will truncate testcases to it so it does not cause
  buffer overflows, and will write the actual size of the input here each iteration
  so we know how much data the fuzzer has given us.

Then, we let the function we are testing run normally. If a CPU exception happens, the
fuzzer will pick it up and treat the input as a "solution" that triggers a configured
exceptional condition.

Finally, we check the status of certificate verification. If validation was successful,
we `HARNESS_ASSERT` because we *really* do not expect this to happen, and we want to
know if it does happen. This type of assertion can be used for any condition that you
want to fuzz for in your code. If the status is a certificate verification failure, we
`HARNESS_STOP`, which just tells the fuzzer we completed our test under normal
conditions and we should run again.

Re-compile the application by running the build script.


## Obtain a Corpus

The fuzzer will take input from the `corpus` directory in the project directory, so
we'll create that directory and add some sample certificate files in DER format as
our input corpus.

```sh
mkdir corpus
curl -L -o corpus/0 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/0
curl -L -o corpus/1 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/1
curl -L -o corpus/2 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/2
curl -L -o corpus/3 https://github.com/dvyukov/go-fuzz-corpus/raw/master/x509/certificate/corpus/3
```

## Configuring the Fuzzer

Even though we loaded the fuzzer module, it didn't run previously because we did not
instantiate and configure it. Let's do that now. At the top of your `run.simics`
script, we'll add each of the following lines.

First, we need to create an actual `tsffs` object to instantiate the fuzzer.

```simics
load-module tsffs # You should already have this
init-tsffs
```

Next, we'll set the log level to maximum for demonstration purposes:

```simics
tsffs.log-level 4
```

Then, we'll set the fuzzer to start and stop on the magic harnesses we just compiled
into our UEFI application. This is the default, so these calls can be skipped in real
usage unless you want to change the defaults, they are just provided here for
completeness.

```simics
@tsffs.start_on_harness = True
@tsffs.stop_on_harness = True
```

We'll set up our "solutions" which are all the exceptional conditions that we want to
fuzz for. In our case, these are timeouts (we'll set the timeout to 3 seconds) to detect
hangs, and CPU exceptions. we'll enable exceptions 13 for general protection fault and
14 for page faults to detect out of bounds reads and writes.

```simics
@tsffs.timeout = 3.0
@tsffs.exceptions = [13, 14]
```

We'll tell the fuzzer where to take its corpus and save its solutions. The fuzzer will
take its corpus from the `corpus` directory and save solutions to the `solutions`
directory in the project by default, so this call can be skipped in real usage unless
you want to change the defaults.

```simics
@tsffs.corpus_directory = SIM_lookup_file("%simics%/corpus")
@tsffs.solutions_directory = SIM_lookup_file("%simics%/solutions")
```

We'll also *delete* the following code from the `run.simics` script:

```simics
script-branch {
  bp.time.wait-for seconds = 30
  quit 0
}
```

Since we'll be fuzzing, we don't want to exit!
