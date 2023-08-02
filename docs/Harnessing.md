# Harnessing

This document outlines options for harnessing your code for fuzzing. The objective of
this project is to require as little change to your target software as possible to begin
the fuzzing process. Please file an issue if you have a use case that cannot be
satisfied by any of the approaches outlined here.

- [Harnessing](#harnessing)
  - [Harnessing With Provided Include File](#harnessing-with-provided-include-file)
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

- Add the `-I /path/to/applications.fuzzing.security.confuse/include/` flag to your
  compile command and add the line `#include "tsffs.h"` to your target
- Copy `tsffs.h` into your source code next to the target file you are harnessing
  and add the line `#include "tsffs.h"`
- Add the line
  `#include "/path/to/applications.fuzzing.security.confuse/include/tsffs.h"` to your 
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
HARNESS_START(&buffer[0], &size);
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

## Harnessing In Non-C Languages

If your target is written in Rust, you can depend on the `include` crate in this
repository and utilize the architecture-specific harness functions.

For example, in your `Cargo.toml`, you can add the following to depend on the crate.
Note the feature version must match a version of SIMICS base you have installed, and
preferably should match the version you will use when fuzzing.

```toml
[dependencies]
include = { path = "/path/to/applications.fuzzing.security.confuse/include/", features = ["6.0.168" ]}
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