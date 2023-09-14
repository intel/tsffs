# Harnessing

This document outlines options for harnessing your code for fuzzing. The objective of
this project is to require as little change to your target software as possible to begin
the fuzzing process. Please file an issue if you have a use case that cannot be
satisfied by any of the approaches outlined here.

- [Harnessing](#harnessing)
  - [Harnessing With Provided Include File](#harnessing-with-provided-include-file)
  - [Harnessing in Constrained UEFI Environments](#harnessing-in-constrained-uefi-environments)
  - [Harnessing In Non-C Languages](#harnessing-in-non-c-languages)


## Harnessing With Provided Include File

In the [first tutorial](./UEFISimpleTarget.md), we wrote inline assembly to invoke the
magic `cpuid` instruction that signals the start and end of the harness. In practice, we
recommend you use the header file we provide to harness your code, either by
`#include`-ing it or, if that is difficult due to your build system, by pasting its
contents into the file containing the code you are harnessing.

The include file [tsffs.h](../include/tsffs.h) is in the [include](../include/)
directory of this repository. To include it, you can use any of the below approaches, in
rough order of best to worst.

- Add the `-I /path/to/tsffs/include/` flag to your
  compile command and add the line `#include "tsffs.h"` to your target
- Copy `tsffs.h` into your source code next to the target file you are harnessing
  and add the line `#include "tsffs.h"`
- Add the line
  `#include "/path/to/tsffs/include/tsffs.h"` to your
  source code in the target file.
- Copy and paste the contents of `tsffs.h` into your source file.

Using this header, the start harness:

```c
__asm__ __volatile__(
    "cpuid\n\t"
    : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d), "=S"(buffer_ptr), "=D"(size_ptr)
    : "0"((0x0001U << 16U) | 0x4711U), "S"(buffer_ptr), "D"(size_ptr));
```

Becomes simply:

```c
HARNESS_START(&buffer_ptr, &size);
```

And the stop harness:


```c
__asm__ __volatile__("cpuid\n\t"
                      : "=a"(_a), "=b"(_b), "=c"(_c), "=d"(_d)
                      : "0"((0x0002U << 16U) | 0x4711U));
```

Becomes:

```c
HARNESS_STOP();
```

## Harnessing in Constrained UEFI Environments

Some UEFI build environments will not support GNU inline assembly or the `__cpuid` and
`__cpuidex` macros used with the MSVC compiler. In these instances, you can write your
own harness functions if you have another method of causing a `cpuid` instruction. For
example, if you have functions defined by your build environment:

```c
void CpuId(UINT32 CpuInfo[4], UINT32 FunctionId);
void CpuIdEx(UINT32 CpuInfo[4], UINT32 FunctionId, UINT32 SubFunctionId);
```

You can create your own harness code like the below:

```c
#define MAGIC_START 1
#define MAGIC_STOP 2
#define MAGIC_START_WININTRIN 3
#define MAGIC 18193

void harness_start(unsigned char **addr_ptr, unsigned long long *size_ptr) {
  unsigned int cpuInfo[4] = {0};
  unsigned int function_id_start = (MAGIC_START_WININTRIN << 16U) | MAGIC;
  unsigned int subfunction_id_addr_low =
      (unsigned int)(((unsigned long long)*addr_ptr) & 0xffffffff);
  unsigned int subfunction_id_addr_hi =
      (unsigned int)(((unsigned long long)*addr_ptr) >> 32U);
  unsigned int subfunction_id_size_low =
      (unsigned int)(((unsigned long long)*size_ptr) & 0xffffffff);
  unsigned int subfunction_id_size_hi =
      (unsigned int)(((unsigned long long)*size_ptr) >> 32U);
  CpuIdEx(cpuInfo, function_id_start, subfunction_id_addr_low);
  CpuIdEx(cpuInfo, function_id_start, subfunction_id_addr_hi);
  CpuIdEx(cpuInfo, function_id_start, subfunction_id_size_low);
  CpuIdEx(cpuInfo, function_id_start, subfunction_id_size_hi);
  *(long long *)addr_ptr = 0;
  *(long long *)addr_ptr |= (long long)cpuInfo[0];
  *(long long *)addr_ptr |= ((long long)cpuInfo[1]) << 32U;
  *(long long *)size_ptr = 0;
  *(long long *)size_ptr |= (long long)cpuInfo[2];
  *(long long *)size_ptr |= ((long long)cpuInfo[3]) << 32U;
}

void harness_stop(void) {
  unsigned int cpuInfo[4] = {0};
  unsigned int function_id_stop = (MAGIC_STOP << 16U) | MAGIC;
  CpuId(cpuInfo, function_id_stop);
}
```

## Harnessing In Non-C Languages

If your target is written in Rust, you can depend on the `include` crate in this
repository and utilize the architecture-specific harness functions.

For example, in your `Cargo.toml`, you can add the following to depend on the crate.
Note the feature version must match a version of SIMICS base you have installed, and
preferably should match the version you will use when fuzzing.

```toml
[dependencies]
include = { path = "/path/to/tsffs/include/", features = ["6.0.169" ]}
```

Then, you can call the harness functions:


```rust
use include::x86_64::{harness_start, harness_stop};

fn main() {
  let mut buf = &[0; 32];
  let mut buf_ptr = buf.as_mut_ptr();
  let mut buf_size: u64 = buf.len().try_into().unwrap();
  harness_start(&mut buf_ptr, &mut buf_size);
  // Your code here, using buf/buf_size!
  harness_stop();
}
```

If your target is written in pure assembly, you can directly write the equivalent of the
inline assembly shown above. Consult the SIMICS documentation (installed with your
SIMICS installation) for reference on the magic instructions for each architecture, and
pass `n=1` to signal a start and `n=2` to signal a stop. The registers used for
auxiliary information are different for each architecture and are defined by the fuzzer
module in [magic/mod.rs](../tsffs_module/src/magic/mod.rs).