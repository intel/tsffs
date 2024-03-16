# Create a Fuzz Harness

We'll create a directory `~/fuzzer` where we'll create and run our fuzz harness:

```powershell
mkdir ~/fuzzer
cd ~/fuzzer
```

We're going to use LibFuzzer to fuzz the driver via its IOCTL interface. The handler for
the interface is defined
[here](https://github.com/novafacing/HackSysExtremeVulnerableDriver/blob/master/Driver/HEVD/Windows/BufferOverflowStack.c).

Essentially, if we pass more than `512 * 4 = 2048` bytes of data, we will begin to
overflow the stack buffer. Create `fuzzer.c` by running `vim fuzzer.c`.

We'll start by including `windows.h` for the Windows API and `stdio.h` so we can print.

```c
#include <windows.h>
#include <stdio.h>
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

Next, we'll define our device name, a handle for the device once we open it, and a
global flag to check whether the device is initialized/opened. We could have also used
the [Startup initialization](https://llvm.org/docs/LibFuzzer.html#startup-initialization)
interface to LibFuzzer, but since we don't really need access to `argv`, we follow the
guidance to use a statically initialized global object.

```c
const char g_devname[] = "\\\\.\\HackSysExtremeVulnerableDriver";
HANDLE g_device = INVALID_HANDLE_VALUE;
BOOL g_device_initialized = FALSE;
```

Now we can declare our fuzz driver. Since we're compiling as C code, note we do not
declare the function as `extern "C"`, but if we were compiling as C++ we would need to
do this.

```c
int LLVMFuzzerTestOneInput(const BYTE *data, size_t size) {
```

This function will be called in a loop with new inputs each time by the fuzz driver.

The first thing we need to do is check if the device handle is initialized, and
initialize it if not.

```c
    if (!g_device_initialized) {
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
        g_device_initialized = TRUE;
    }
```

Returning `-1` from `LLVMFuzzerTestOneInput` indicates a bad input, but we will also
use it here to indicate an initialization error.

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
        (BYTE *)data,
        (DWORD)size,
        NULL, //outBuffer -> None
        0, //outBuffer size -> 0
        &size_returned,
        NULL
    );

    if (!is_ok) {
        printf("Error in DeviceIoControl\n");
        return -1;
    }

    return 0;
}
```
