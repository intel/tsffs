# Create a Fuzz Harness

We'll create a directory `~/fuzzer` where we'll create and run our fuzz harness:

```powershell
mkdir ~/fuzzer
cd ~/fuzzer
```

We're going to fuzz the driver via its IOCTL interface. The handler for the interface is
defined
[here](https://github.com/novafacing/HackSysExtremeVulnerableDriver/blob/master/Driver/HEVD/Windows/BufferOverflowStack.c).
It is possible as well to harness the kernel driver directly, but it is typically much
easier to use a user-space driver to fuzz the kernel driver. This has the added benefit
that most test cases for drivers are implemented as user-space programs, so converting a
test case to a fuzz driver becomes very simple.

Essentially, if we pass more than `512 * 4 = 2048` bytes of data, we will begin to
overflow the stack buffer. Create `fuzzer.c` by running `vim fuzzer.c`.

We'll start by including `windows.h` for the Windows API and `stdio.h` so we can print.

```c
#include <windows.h>
#include <stdio.h>
```

We will also include our TSFFS header:

```c
#include "tsffs.h"
```

Next, we need to define the [control
code](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d4drvif/nf-d4drvif-ctl_code)
for the driver interface. The device servicing IOCTL we are triggering is not a
pre-specified [file
type](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/specifying-device-types),
so we access it with an unknown type. We grab the control code for the handler we want
from the [driver source
code](https://github.com/novafacing/HackSysExtremeVulnerableDriver/blob/master/Driver/HEVD/Windows/HackSysExtremeVulnerableDriver.h).
Note that in the handler, we can see this is a type 3 IOCTL handler (AKA
`METHOD_NEITHER`) and that we want RW access to the driver file.

```c
#define HACKSYS_EVD_IOCTL_STACK_OVERFLOW CTL_CODE(FILE_DEVICE_UNKNOWN, 0x800, METHOD_NEITHER, FILE_ANY_ACCESS)
```

Next, we'll define our device name and a handle for the device once we open it.

```c
const char g_devname[] = "\\\\.\\HackSysExtremeVulnerableDriver";
HANDLE g_device = INVALID_HANDLE_VALUE;
```

Now we can implement our fuzz driver. Since we're compiling as C code, note we do not
declare the function as `extern "C"`, but if we were compiling as C++ we would need to
do this.

```c
int main() {
```

The first thing we need to do is check if the device handle is initialized, and
initialize it if not.

```c
    printf("Initializing device\n");

    if ((g_device = CreateFileA(g_devname,
        GENERIC_READ | GENERIC_WRITE,
        0,
        NULL,
        OPEN_EXISTING,
        0,
        NULL
    )) == INVALID_HANDLE_VALUE) {
        printf("Failed to initialize device\n");
        return -1;
    }
    printf("Initialized device\n");
```

Next, we'll declare a buffer and a size of the buffer. We'll make it 1 page in size.
Note that the `size` variable must be a pointer-width integer to be compatible with the
TSFFS fuzz harnesses. We will downcast it to the DWORD size parameter for
`DeviceIoControl` later.

```c
   BYTE buffer[4096];
   size_t size = 4096;
```

Now we can add our start harness. When this harness function executes, the fuzzer will
take a snapshot. Each fuzzing iteration, `buffer` will be filled with up to 4096 bytes
of fuzzer data and `size` will be set to the actual number of bytes of the fuzzing
testcase.

```c
    HARNESS_START(buffer, &size);
```

We'll also add a print for ourselves to let us know if the buffer *should* be overflowed
by an input.

```c
    if (size > 2048) {
        printf("Overflowing buffer!\n");
    }
```

Finally, we'll call `DeviceIoControl` to interact with the driver by passing our input
data to the IOCTL interface.

```c
    DWORD size_returned = 0;

    BOOL is_ok = DeviceIoControl(g_device,
        HACKSYS_EVD_IOCTL_STACK_OVERFLOW,
        (BYTE *)buffer,
        (DWORD)size,
        NULL, //outBuffer -> None
        0, //outBuffer size -> 0
        &size_returned,
        NULL
    );
```

After executing the IOCTL, we check the return value and in either case, we will add a
`HARNESS_STOP` call, which signals the fuzzer that this fuzzing iteration is over. The
fuzzer will reset to the initial snapshot and run again with a new input. It is
important to insert a stop harness before any exit from the fuzzing harness code path so
the fuzzer knows when to stop. Otherwise, execution will proceed until a timeout occurs,
which in this case would be a false positive.

```c
    if (!is_ok) {
        printf("Error in DeviceIoControl\n");
        HARNESS_STOP();
        return -1;
    }

    HARNESS_STOP();

    return 0;
}
```

Save the file with

## Add Header & ASM File

To build the fuzz harness with the TSFFS harness functions, we need both the header
(`tsffs.h`) and the MSVC ASM file (`tsffs-msvc-x86_64.h`).

Copy `tsffs.h` and `tsffs-msvc-x86_64.h` into the `fuzzer` directory by running the
following on your host machine:

```sh
scp -P 2222 harness/tsffs.h "user@localhost:C:\\Users\\user\\fuzzer\\"
scp -P 2222 harness/tsffs-msvc-x86_64.asm "user@localhost:C:\\Users\\user\\fuzzer\\"
```