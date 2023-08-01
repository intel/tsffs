pub const STOP: u16 = 0x4242;
pub const START: u16 = 0x4343;

#[cfg(any(target_arch = "i386", target_arch = "i586", target_arch = "i686"))]
pub mod i386 {
    pub const MAGIC: u16 = 0x4711;
    #[no_mangle]
    /// X86 32:
    /// cbindgen:prefix= \
    /// #define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1) \ \
    ///     __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \ \
    ///                          "=S"(*inout_ptr_0), "=D"(*inout_ptr_1) \ \
    ///                        : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1) \ \
    ///                        : "memory") \
    /// \
    /// #define __cpuid_extended1(level, a, b, c, d, inout_ptr_0) \ \
    ///     __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \ \
    ///                          "=S"(*inout_ptr_0) \ \
    ///                        : "0"(level), "S"(*inout_ptr_0) \ \
    ///                        : "memory") \
    /// \
    /// #define __cpuid(level, a, b, c, d) \ \
    ///     __asm__ __volatile__("push %%ebx; cpuid; pop %%ebx\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \ \
    ///                        : "0"(level) \ \
    ///                        : "memory") \
    /// \
    /// #define __arch_harness_start(addr_ptr, size_ptr) \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (START << 16U) | MAGIC; \ \
    ///         __cpuid_extended2(leaf, _a, _b, _c, _d, addr_ptr, size_ptr); \ \
    ///     } while (0) \
    /// \
    /// #define __arch_harness_stop() \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (STOP << 16U) | MAGIC; \ \
    ///         __cpuid(leaf, _a, _b, _c, _d); \ \
    ///     } while (0) \
    /// \
    /// #define __arch_harness_stop_extended(val_ptr) \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (STOP << 16U) | MAGIC; \ \
    ///         __cpuid_extended1(leaf, _a, _b, _c, _d, val_ptr); \ \
    ///     } while (0) \
    ///
    pub extern "C" fn __marker_i386() {}
}

#[cfg(target_arch = "x86_64")]
pub mod x86_64 {
    pub const MAGIC: u16 = 0x4711;
    #[no_mangle]
    /// X86_64:
    /// cbindgen:prefix= \
    /// #define __cpuid_extended2(level, a, b, c, d, inout_ptr_0, inout_ptr_1) \ \
    ///     __asm__ __volatile__("cpuid\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \ \
    ///                          "=S"(*inout_ptr_0), "=D"(*inout_ptr_1) \ \
    ///                        : "0"(level), "S"(*inout_ptr_0), "D"(*inout_ptr_1)) \
    /// \
    /// #define __cpuid_extended1(level, a, b, c, d, inout_ptr_0) \ \
    ///     __asm__ __volatile__("cpuid\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d), \ \
    ///                          "=S"(*inout_ptr_0) \ \
    ///                        : "0"(level), "S"(*inout_ptr_0)) \
    /// \
    /// #define __cpuid(level, a, b, c, d) \ \
    ///     __asm__ __volatile__("cpuid\n\t" \ \
    ///                        : "=a"(a), "=b"(b), "=c"(c), "=d"(d) \ \
    ///                        : "0"(level)) \
    /// \
    /// #define __arch_harness_start(addr_ptr, size_ptr) \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (START << 16U) | MAGIC; \ \
    ///         __cpuid_extended2(leaf, _a, _b, _c, _d, addr_ptr, size_ptr); \ \
    ///     } while (0) \
    /// \
    /// #define __arch_harness_stop() \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (STOP << 16U) | MAGIC; \ \
    ///         __cpuid(leaf, _a, _b, _c, _d); \ \
    ///     } while (0) \
    /// \
    /// #define __arch_harness_stop_extended(val_ptr) \ \
    ///     do { \ \
    ///         uint32_t _a __attribute__((unused)) = 0; \ \
    ///         uint32_t _b __attribute__((unused)) = 0; \ \
    ///         uint32_t _c __attribute__((unused)) = 0; \ \
    ///         uint32_t _d __attribute__((unused)) = 0; \ \
    ///         uint32_t leaf = (STOP << 16U) | MAGIC; \ \
    ///         __cpuid_extended1(leaf, _a, _b, _c, _d, val_ptr); \ \
    ///     } while (0) \
    ///
    pub extern "C" fn __marker_x86_64() {}
}

#[cfg(target_arch = "powerpc")]
pub mod powerpc {}

#[cfg(any(target_arch = "powerpc64", target_arch = "powerpc64le"))]
pub mod powerpc64 {}

#[cfg(target_arch = "aarch64")]
pub mod aarch64 {}

#[cfg(any(target_arch = "arm", target_arch = "armv7"))]
pub mod arm {}

#[cfg(any(
    target_arch = "thumbv8m.main",
    target_arch = "thumbv8m.base",
    target_arch = "thumbv7neon",
    target_arch = "thumbv7m",
    target_arch = "thumbv7em",
    target_arch = "thumbv7a",
))]
pub mod thumb {}

#[cfg(any(
    target_arch = "mips",
    target_arch = "mipsel",
    target_arch = "mips64",
    target_arch = "mips64el",
))]
pub mod mips {}

#[cfg(any(
    target_arch = "riscv32gc",
    target_arch = "riscv32i",
    target_arch = "riscv32im",
    target_arch = "riscv32imac",
    target_arch = "riscv32imc",
    target_arch = "riscv64gc",
    target_arch = "riscv64imac",
))]
pub mod riscv {}

pub mod harness {
    #[no_mangle]
    /// Architecture-independent harness macros:
    ///
    /// cbindgen:prefix= \
    /// #define HARNESS_START(addr_ptr, size_ptr) \ \
    ///     do { \ \
    ///         __arch_harness_start(addr_ptr, size_ptr); \ \
    ///     } while (0) \
    /// \
    /// #define HARNESS_STOP()  \ \
    ///     do { \ \
    ///         __arch_harness_stop(); \ \
    ///     } while (0) \
    /// \
    /// #define HARNESS_STOP_EXTENDED(val_ptr) \ \
    ///     do { \ \
    ///         __arch_harness_stop_extended(val_ptr); \ \
    ///     } while (0) \
    ///
    pub extern "C" fn __marker() {}
}
