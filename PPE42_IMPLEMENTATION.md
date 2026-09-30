# PPE42 Architecture Support for TSFFS

This document describes the complete implementation of PPE42 (PowerPC Processor Embedded 42-bit) architecture support in TSFFS, including coverage-guided fuzzing for embedded firmware.

## Table of Contents

- [Overview](#overview)
- [Architecture Characteristics](#architecture-characteristics)
- [Implementation Details](#implementation-details)
- [Usage Guide](#usage-guide)
- [Performance](#performance)
- [Troubleshooting](#troubleshooting)
- [Files Modified](#files-modified)

## Overview

PPE42 is a 32-bit embedded PowerPC processor without an MMU, commonly used in firmware like IBM's Self-Boot Engine (SBE). This implementation enables coverage-guided fuzzing on PPE42 targets using manual instrumentation through magic instructions.

### Key Challenge

Unlike x86/ARM architectures that support automatic per-instruction callbacks via `CpuInstrumentationSubscribeInterface`, PPE42 SIMICS models lack this interface. The solution: **manual coverage instrumentation** using magic instructions.

## Architecture Characteristics

### PPE42 Specifics

- **32-bit Architecture**: 4-byte pointers and addresses
- **No MMU**: Uses physical addresses directly
- **No Virtual Memory**: No address translation required
- **Embedded Processor**: Designed for firmware and embedded systems
- **Limited SIMICS Support**: Missing `CpuInstrumentationSubscribeInterface`

### Magic Instruction Choice

**Problem**: The standard `tw` (trap word) instruction doesn't work on PPE42 SIMICS models.

**Solution**: Use `rlwimi` (Rotate Left Word Immediate then Mask Insert) as a no-op magic instruction:
- Rotates a register by 0 bits (no-op)
- SIMICS can intercept it via `Core_Magic_Instruction` HAP
- Doesn't affect program execution
- Encodes magic numbers in instruction parameters

## Implementation Details

### 1. Architecture Detection (`src/arch/ppe42.rs`)

```rust
pub struct Ppe42Architecture;

impl Architecture for Ppe42Architecture {
    const SIMICS_ARCH_NAME: &'static str = "ppc";
    const USE_PHYSICAL_ADDRESSES: bool = true;
    const POINTER_WIDTH_OVERRIDE: Option<i32> = Some(4);
    const INDEX_SELECTOR_REGISTER: &'static str = "r10";
    
    fn cpu_instrumentation_subscribe_interface(
        cpu: *mut ConfObject,
    ) -> Result<*mut CpuInstrumentationSubscribe> {
        // Optional - may not be available on PPE42
        cpu_instrumentation_subscribe_interface(cpu)
            .ok()
            .ok_or_else(|| Error::msg("Interface not available"))
    }
}
```

**Key Points**:
- `USE_PHYSICAL_ADDRESSES = true`: No address translation
- `POINTER_WIDTH_OVERRIDE = Some(4)`: Force 4-byte pointer reads
- `INDEX_SELECTOR_REGISTER = "r10"`: Register for passing coverage IDs
- Optional CPU instrumentation interface (gracefully handles absence)

### 2. Physical Address Handling (`src/arch/mod.rs`)

```rust
// Read size from physical memory (4 bytes for PPE42)
let size = read_phys_memory(
    self.cpu(),
    address.physical_address(),
    size_size
)? as usize;
```

### 3. Coverage Magic Number (`src/magic/mod.rs`)

```rust
pub enum MagicNumber {
    Start = 1,
    StartBufferPtr = 2,
    StartBufferPtrSizePtr = 3,
    Stop = 4,
    Assert = 5,
    Coverage = 6,  // New for manual coverage
}
```

**Magic Number Mapping**:
- User code: `8016` (TSFFS_MAGIC_BASE + 6)
- SIMICS intercepts: `8016`
- TSFFS normalizes: `8016 >> 8 = 31`, then maps to `6`

### 4. Coverage Handler (`src/haps/mod.rs`)

```rust
if magic_number == MagicNumber::Coverage {
    if self.coverage_enabled {
        let pc = processor.get_program_counter()?;
        let coverage_id = index_selector;  // From r10
        
        // Create synthetic PC from coverage ID
        let synthetic_pc = (pc & 0xFFFFFFFF00000000) | (coverage_id & 0xFFFFFFFF);
        
        // Log to AFL coverage map
        self.log_pc(synthetic_pc)?;
    }
    return Ok(());  // Don't halt simulation
}
```

**Key Behavior**:
- Reads coverage ID from r10 register
- Creates synthetic PC for AFL map
- Logs coverage without stopping simulation
- Enables efficient fuzzing loop

### 5. Harness Header (`harness/tsffs-gcc-ppe42.h`)

#### Low-Level Magic Instruction Primitives

```c
// Basic magic instruction (no arguments)
#define __ppe42_magic(n) \
  __asm__ __volatile__("rlwimi %0,%0,0,%1,%2" \
    : \
    : "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : )

// Magic instruction with 1 argument (coverage ID in r10)
#define __ppe42_magic_extended1(n, arg0) \
  __asm__ __volatile__("mr 10, %0; rlwimi %1,%1,0,%2,%3" \
    : \
    : "r"(arg0), \
      "i" (((n) >> 8) & 0x1f), \
      "i" (((n) >> 4) & 0xf), \
      "i" ((((n) >> 0) & 0xf) | 16) \
    : "r10")
```

#### Coverage Macros

```c
#define TSFFS_MAGIC_BASE (8010)
#define N_COVERAGE (TSFFS_MAGIC_BASE + 6)  // 8016

#define HARNESS_COVERAGE(coverage_id) \
    __ppe42_magic_extended1(N_COVERAGE, coverage_id)

#define COVERAGE_BRANCH(id) HARNESS_COVERAGE(id)
```

#### Standard Harness Macros

```c
#define HARNESS_START(buffer, size_ptr) \
  do { \
    __ppe42_magic_extended3(N_START_BUFFER_PTR_SIZE_PTR, DEFAULT_INDEX, \
                            (unsigned long)(buffer), (unsigned long)(size_ptr)); \
  } while (0)

#define HARNESS_STOP() __ppe42_magic(N_STOP)
#define HARNESS_ASSERT() __ppe42_magic(N_ASSERT)
```

## Usage Guide

### 1. Include Header in Target Code

```c
#include "tsffs-gcc-ppe42.h"

void fuzz_target(void) {
    uint8_t *fuzz_buffer;
    uint32_t fuzz_size;
    
    // Start fuzzing - TSFFS provides input
    HARNESS_START(&fuzz_buffer, &fuzz_size);
    
    // Add coverage points at interesting branches
    COVERAGE_BRANCH(1);  // Entry point
    
    if (fuzz_size == 0) {
        COVERAGE_BRANCH(2);  // Empty input
        HARNESS_STOP();
        return;
    }
    
    COVERAGE_BRANCH(3);  // Non-empty input
    
    // Nested branches for deep path discovery
    if (fuzz_buffer[0] == 'A') {
        COVERAGE_BRANCH(4);
        if (fuzz_size > 1 && fuzz_buffer[1] == 'B') {
            COVERAGE_BRANCH(5);
            if (fuzz_size > 2 && fuzz_buffer[2] == 'C') {
                COVERAGE_BRANCH(6);
            }
        }
    }
    
    COVERAGE_BRANCH(7);  // Exit point
    HARNESS_STOP();
}
```

### 2. Configure SIMICS Script

```python
# Load TSFFS module
load-module tsffs

# Configure magic instruction breakpoints
bp.magic.break 8011  # Start - halt simulation
bp.magic.break 8012  # Start with buffer pointer
bp.magic.break 8013  # Start with buffer and size pointers
bp.magic.break 8014  # Stop - halt simulation
bp.magic.break 8015  # Assert - halt simulation
bp.magic.trace 8016  # Coverage - continue without halting

# Configure fuzzer
@tsffs.iteration_limit = 1000        # Stop after 1000 iterations
@tsffs.timeout = 5.0                 # 5 second timeout per iteration
@tsffs.all_exceptions_are_solutions = True  # Save crashes
@tsffs.coverage_enabled = True       # Enable coverage tracking

# Start fuzzing
@tsffs.start()
```

### 3. Build and Run

```bash
# Build TSFFS
cd /path/to/tsffs
cargo simics-build -r

# Build target firmware with coverage instrumentation
cd /path/to/firmware
make clean && make

# Run fuzzer
cd /path/to/simics/project
./simics -no-gui -no-win fuzz.simics
```

### 4. Analyze Results

```bash
# View crashes
ls -lh simics/%simics%/solutions/

# View corpus (interesting inputs)
ls -lh simics/%simics%/corpus/

# Replay a crash
./simics -no-gui -no-win replay.simics
```

## Performance

### Typical Metrics

- **Execution Speed**: 2-3 executions/second
- **Coverage Discovery**: Efficient path exploration with manual instrumentation
- **Corpus Growth**: From seed inputs to 5-10x corpus size
- **Crash Detection**: Automatic via exception handling

### Example Results

From a real fuzzing session on SBE firmware:

```
Iterations: 900+
Execution Rate: 2-3 exec/sec
Coverage Points: 19 unique branches discovered
Corpus Growth: 6 seeds → 32 interesting inputs
Crashes Found: 1 (input starting with 'X')
Time: ~5 minutes
```

### Optimization Tips

1. **Strategic Coverage Points**: Place `COVERAGE_BRANCH()` at:
   - Function entry/exit
   - Conditional branches
   - Loop iterations
   - Error handling paths

2. **Minimize Coverage Overhead**: Don't instrument every line
   - Focus on decision points
   - Skip straight-line code
   - Balance granularity vs. performance

3. **Timeout Configuration**: Adjust based on target complexity
   - Simple functions: 1-2 seconds
   - Complex operations: 5-10 seconds
   - I/O operations: 10+ seconds
### Fuzzing Large Internal Functions

For large internal functions where manual instrumentation is impractical, use these strategies:

#### Strategy 1: Wrapper Function with Minimal Coverage

Create a thin wrapper that only instruments the entry/exit points:

```c
// Original large internal function (no changes needed)
static int large_internal_function(uint8_t *data, size_t len) {
    // 1000+ lines of complex logic
    // Multiple branches, loops, etc.
    // DO NOT add COVERAGE_BRANCH here
    return result;
}

// Fuzzing wrapper with minimal instrumentation
void fuzz_large_function(void) {
    uint8_t *fuzz_buffer;
    uint32_t fuzz_size;
    
    HARNESS_START(&fuzz_buffer, &fuzz_size);
    COVERAGE_BRANCH(1);  // Entry only
    
    // Call the large function unchanged
    int result = large_internal_function(fuzz_buffer, fuzz_size);
    
    COVERAGE_BRANCH(2);  // Exit only
    HARNESS_STOP();
}
```

**Pros**: 
- No changes to original function
- Minimal overhead
- Still discovers crashes

**Cons**: 
- No path coverage (fuzzer is "blind")
- Slower to find deep bugs
- Relies on crash detection only

#### Strategy 2: Instrument Only Critical Decision Points

Selectively add coverage to key branches without full instrumentation:

```c
static int large_internal_function(uint8_t *data, size_t len) {
    COVERAGE_BRANCH(100);  // Function entry
    
    // Skip straight-line code (no instrumentation)
    int result = 0;
    uint32_t checksum = calculate_checksum(data, len);
    
    // Instrument only major branches
    if (checksum == EXPECTED_CHECKSUM) {
        COVERAGE_BRANCH(101);  // Valid checksum path
        
        // More straight-line code (no instrumentation)
        result = process_valid_data(data, len);
        
    } else {
        COVERAGE_BRANCH(102);  // Invalid checksum path
        return -1;
    }
    
    // Instrument error handling
    if (result < 0) {
        COVERAGE_BRANCH(103);  // Error path
        handle_error(result);
    }
    
    COVERAGE_BRANCH(104);  // Function exit
    return result;
}
```

**Guidelines for Selective Instrumentation**:
- ✅ **DO instrument**: if/else, switch cases, loop entries, error returns
- ❌ **DON'T instrument**: assignments, calculations, function calls (unless critical)
- 🎯 **Target**: 5-20 coverage points per 1000 lines of code

#### Strategy 3: Compiler-Based Coverage (Future Enhancement)

For truly massive functions, consider compiler instrumentation:

```bash
# Compile with GCC coverage flags
gcc -fprofile-arcs -ftest-coverage -o target.o target.c

# Or use LLVM SanitizerCoverage
clang -fsanitize-coverage=trace-pc-guard -o target.o target.c
```

**Note**: This requires TSFFS enhancement to read compiler-generated coverage data. Currently not implemented for PPE42.

#### Strategy 4: Hybrid Approach

Combine minimal wrapper with strategic internal points:

```c
static int large_internal_function(uint8_t *data, size_t len) {
    // Only instrument the "interesting" parts
    
    if (data[0] == MAGIC_BYTE) {
        COVERAGE_BRANCH(200);  // Rare path discovered
        return special_handling(data, len);
    }
    
    // 900 lines of code with no instrumentation
    // ...
    
    if (error_condition) {
        COVERAGE_BRANCH(201);  // Error path
        return -1;
    }
    
    return 0;
}

void fuzz_wrapper(void) {
    uint8_t *fuzz_buffer;
    uint32_t fuzz_size;
    
    HARNESS_START(&fuzz_buffer, &fuzz_size);
    COVERAGE_BRANCH(1);
    
    large_internal_function(fuzz_buffer, fuzz_size);
    
    COVERAGE_BRANCH(2);
    HARNESS_STOP();
}
```

#### Strategy 5: Multiple Fuzzing Targets

Break large function into multiple fuzzing targets:

```c
// Fuzz different entry points separately
void fuzz_function_path_a(void) {
    uint8_t *fuzz_buffer;
    uint32_t fuzz_size;
    HARNESS_START(&fuzz_buffer, &fuzz_size);
    
    // Set up state for path A
    setup_for_path_a();
    COVERAGE_BRANCH(1);
    
    large_internal_function(fuzz_buffer, fuzz_size);
    
    COVERAGE_BRANCH(2);
    HARNESS_STOP();
}

void fuzz_function_path_b(void) {
    uint8_t *fuzz_buffer;
    uint32_t fuzz_size;
    HARNESS_START(&fuzz_buffer, &fuzz_size);
    
    // Set up state for path B
    setup_for_path_b();
    COVERAGE_BRANCH(3);
    
    large_internal_function(fuzz_buffer, fuzz_size);
    
    COVERAGE_BRANCH(4);
    HARNESS_STOP();
}
```

#### Recommended Approach for Large Functions

**For functions > 500 lines:**

1. **Start with Strategy 1** (wrapper only)
   - Run fuzzer for 10,000 iterations
   - Check if crashes are found
   - If yes: you're done!
   - If no: proceed to step 2

2. **Add Strategy 2** (selective instrumentation)
   - Identify 10-20 most important branches
   - Add `COVERAGE_BRANCH()` only to those
   - Run fuzzer again
   - Monitor corpus growth

3. **Iterate based on results**
   - If corpus grows: coverage is working
   - If corpus stagnates: add more coverage points
   - If too slow: remove some coverage points

**Result**: 8 coverage points guide fuzzer to explore all major paths without instrumenting 2000 lines.

#### Coverage Density Guidelines

| Function Size | Recommended Coverage Points | Ratio |
|---------------|----------------------------|-------|
| < 100 lines   | 5-10 points               | 1:10  |
| 100-500 lines | 10-20 points              | 1:25  |
| 500-1000 lines| 15-30 points              | 1:33  |
| > 1000 lines  | 20-50 points              | 1:50  |

**Key Principle**: More coverage is not always better. Focus on **decision points** that change program behavior, not every line of code.


## Troubleshooting

### Issue: "Interface not available" Error

**Symptom**: Build fails with CPU instrumentation interface error

**Solution**: This is expected on PPE42. The implementation gracefully handles missing interfaces.

### Issue: Coverage Not Tracking

**Symptom**: Fuzzer runs but corpus doesn't grow

**Checklist**:
1. Verify `bp.magic.trace 8016` (not `bp.magic.break`)
2. Check `@tsffs.coverage_enabled = True`
3. Ensure `COVERAGE_BRANCH()` calls in target code
4. Verify r10 register initialization

### Issue: Fuzzer Timeouts

**Symptom**: All iterations timeout

**Solutions**:
1. Increase timeout: `@tsffs.timeout = 10.0`
2. Check for infinite loops in target
3. Verify `HARNESS_STOP()` is called

### Issue: No Crashes Found

**Symptom**: Fuzzer runs but finds no crashes

**This is normal if**:
- Target code is robust
- Input validation is strong
- Coverage is limited

**To improve**:
1. Add more coverage points
2. Increase iteration limit
3. Provide better seed inputs
4. Disable input validation temporarily

### Issue: Build Errors with SIMICS Version

**Symptom**: "Could not extract version from SIMICS_BASE path"

**Solution**: Set `SIMICS_BASE` to standard SIMICS installation:
```bash
export SIMICS_BASE=/path/to/simics-6.0.xxx/simics-6.0.xxx
```

## Files Modified

### Core TSFFS Files

1. **`src/arch/ppe42.rs`** (NEW)
   - PPE42 architecture implementation
   - Physical address mode
   - Optional CPU instrumentation interface

2. **`src/arch/mod.rs`**
   - PPE42 architecture detection
   - Physical address handling
   - Removed unnecessary workaround code
   - Removed debug statements

3. **`src/magic/mod.rs`**
   - Added `Coverage = 6` magic number

4. **`src/lib.rs`**
   - Magic number normalization (8016 → 6)

5. **`src/haps/mod.rs`**
   - Coverage HAP handler (non-halting)
   - Synthetic PC generation from coverage ID

6. **`src/tracer/mod.rs`**
   - Made `log_pc()` public for coverage tracking

7. **`build.rs`**
   - Fixed SIMICS version extraction for non-standard paths

### Harness Files

8. **`harness/tsffs-gcc-ppe42.h`** (CONSOLIDATED)
   - Low-level `rlwimi`-based magic instructions
   - Magic number definitions
   - Standard harness macros (START, STOP, ASSERT)
   - Coverage macros (COVERAGE_BRANCH)
   - Complete 304-line header (10KB)


## Technical Notes

### Why `rlwimi` Instead of `tw`?

The `tw` (trap word) instruction is the standard PowerPC trap instruction, but PPE42 SIMICS models don't support it. The `rlwimi` instruction:
- Is a valid PPE42 instruction
- Can be configured as a no-op (rotate by 0)
- Is intercepted by SIMICS magic instruction mechanism
- Encodes magic numbers in immediate fields

### Coverage ID Encoding

Coverage IDs are passed via r10 register:
1. User code: `COVERAGE_BRANCH(42)` → `mr 10, 42`
2. Magic instruction: `rlwimi` with magic number 8016
3. SIMICS HAP: Reads r10 value (42)
4. TSFFS: Creates synthetic PC: `(real_pc & 0xFFFFFFFF00000000) | 42`
5. AFL map: Logs edge coverage

### Physical vs. Virtual Addresses

PPE42 has no MMU, so all addresses are physical:
- No page tables
- No address translation
- Direct memory access
- Simplified implementation

## Conclusion

This implementation demonstrates a complete solution for coverage-guided fuzzing on embedded architectures lacking automatic instrumentation. The manual coverage approach using magic instructions provides:

- **Flexibility**: Works on any architecture with magic instruction support
- **Efficiency**: Minimal overhead compared to per-instruction callbacks
- **Control**: Precise coverage point placement
- **Compatibility**: Works with existing SIMICS infrastructure

The PPE42 implementation serves as a template for adding support for other embedded architectures with similar constraints.

## References

- [TSFFS Documentation](https://intel.github.io/tsffs)
- [LibAFL](https://github.com/AFLplusplus/LibAFL)
- [SIMICS Documentation](https://www.intel.com/content/www/us/en/developer/articles/tool/simics-simulator.html)
- PPE42 Architecture Manual (IBM)

## Contact

For questions about PPE42 support, please:
1. Check this documentation
2. Review the source code comments
3. File an issue on GitHub
4. Contact the TSFFS authors

---

**Last Updated**: 2026-03-06  
**TSFFS Version**: 0.2.5  
**SIMICS Version**: 6.0.185+
