# 1BRC Implementation Deep Dive

This document explains the implementation details of our One Billion Row Challenge (1BRC) solution in Rust.

This solution aggressively optimizes for performance over idiomatic safe Rust. It steps deeply into "systems
programming" territory, utilizing direct memory management via OS syscalls, raw pointer arithmetic, SIMD (Single
Instruction, Multiple Data) intrinsics, custom prefix-free hashing, and branchless arithmetic.

## High-Level Architecture

Instead of a linear read, the program follows a **Map-Reduce** concurrency model:

1. **Memory Map:** Ask the OS to map the entire file into our process's virtual memory address space.
2. **Chunking (Map):** Divide the file into exactly $N$ chunks (where $N$ is the number of CPU cores). Adjust chunk
   boundaries to ensure no line is split in half.
3. **Thread-Local Aggregation:** Each thread processes its chunk byte-by-byte using SIMD and branchless parsing,
   aggregating data into a thread-local, heavily optimized Hash Map.
4. **Merge (Reduce):** The main thread receives the thread-local maps via a channel and merges them into a final, sorted
   `BTreeMap`.

---

## 1. I/O: Memory Mapping & OS Advising

The standard way to read a file in Rust is `BufReader`. However, `BufReader` forces the OS to copy data from the
kernel's page cache into a user-space buffer before you can read it.

### `mmap` and Zero-Copy

```rust
let ptr = libc::mmap(..., libc::MAP_SHARED, file.as_raw_fd(), 0);
```

* **What is it?** We bypass user-space buffers entirely. `mmap` tells the OS to map the file's disk addresses directly
  to our RAM.
* **Zero-Copy:** When we index into our `&[u8]` slice, we are reading *directly* from the kernel's page cache.
* **Cache Locality:** Modern CPUs fetch memory in 64-byte chunks (Cache Lines). By reading sequentially from an mmap
  slice, we maximize L1/L2 cache hit rates.

### `MADV_SEQUENTIAL`

```rust
libc::madvise(ptr, len, libc::MADV_SEQUENTIAL);
```

* **The OS Whisperer:** We explicitly advise the Linux kernel about our access pattern. By saying "we will read this
  sequentially," the kernel aggressively pre-fetches (read-ahead) massive chunks of the file from the disk into RAM
  before our application even asks for it, hiding disk I/O latency.

---

## 2. Concurrency: Safe Chunking

To utilize all CPU cores, we divide the memory map into equal-sized byte chunks.

**The Split Problem:** A naive byte-split might cut a line in half (e.g., `Hamb|urg;12.3`).
**The Solution:** Each thread is given a target chunk size. It finds the tentative end of its chunk, then scans
*forward* using `next_newline` to find the very next `\n`. It sets its boundary there, and the next thread starts
exactly at that offset.

This ensures perfect parallelization with zero mutexes or shared state during the hot processing loop.

---

## 3. The Hot Loop: SIMD & Pointer Arithmetic

Inside the thread, we iterate through billions of lines. Slices (`&[u8]`) in Rust carry `(pointer, length)` metadata and
perform bounds checking on every access. In a hot loop, this overhead is fatal.

### Finding Newlines: Fast Path vs Slow Path

```rust
let newline_eq = ascii::DELIMITER_NEW_L.simd_eq(against);
```

Instead of checking bytes one-by-one, we use **SIMD** (`u8x64`).

1. **Fast Path:** We load exactly one Cache Line (64 bytes) into a CPU vector register (`u8x64::load_or_default`). We
   check all 64 bytes for `\n` in a *single CPU cycle*.
2. **Slow Path:** If the line is longer than 64 bytes (very rare), we fall back to `libc::memchr`.
3. **Raw Pointers:** We use `slice.get_unchecked` and pointer math (`offset_from`) to calculate line lengths. This
   proves to the compiler that it doesn't need to emit bounds-checking branches, keeping all variables inside fast CPU
   registers.

### Finding the Delimiter (`;`)

We apply the exact same SIMD strategy for the semicolon. Because we know 1BRC lines are almost never longer than 64
bytes, finding the semicolon becomes a single, parallel CPU instruction.

---

## 4. Branchless Temperature Parsing

Parsing strings to floats (`f64::parse`) is incredibly slow due to IEEE 754 standards, error handling, and Unicode
checks.

**Fixed-Point Math:** We ignore the decimal point. `12.3` becomes `123`. We store everything as `i16` (tenths of a
degree).

**The Pipeline Problem:** CPU architectures rely on "branch prediction" to guess which way an `if` statement will go to
keep the instruction pipeline full. If it guesses wrong (a branch misprediction), it has to flush the pipeline, wasting
10-20 CPU cycles.

**Branchless Execution:**

```rust
let is_positive = ! first_byte_is_minus;
let sign = i16::from(is_positive) * 2 - 1; 
```

Instead of an `if` statement to check for a `-`, we use pure arithmetic.

* If positive: `1 * 2 - 1 = 1`
* If negative: `0 * 2 - 1 = -1`

**Conditional Moves (`cmov`):**
rust
let sign_offset = if first_byte_is_minus { 1 } else { 0 };
While this *looks* like a branch, the Rust compiler optimizes this into a `cmov` assembly instruction. It evaluates both
paths and simply *moves* the correct value into the register based on a CPU flag, entirely avoiding a pipeline-flushing
jump.

By multiplying out the hundreds, tens, and units places using ASCII byte subtraction (`digit - b'0'`), we parse the
temperature in a handful of nanoseconds with zero actual branches.

---

## 5. Hashing & Cache Locality

Rust's default `HashMap` uses SipHash, which is cryptographically secure against HashDoS attacks but heavily penalizes
performance. For 1BRC, we trust the input.

### The `DumbHasher`

```rust
self .0 = word[0] ^ word[1];
self .0 ^ self .0.rotate_right(33) ^ self .0.rotate_right(15)
```

We implemented a custom, highly aggressive hasher:

1. **16-Byte Chunking:** Using `unsafe { std::ptr::copy }`, we copy up to 16 bytes of the station name directly into a
   `[u64; 2]` buffer.
2. **XOR & Rotate:** We XOR the two 64-bit words together, then apply a final mixing step using bitwise rotations. This
   destroys cryptographic security but creates excellent hash distribution in fractions of the time SipHash takes.
3. **Prefix-Free:** We implement `write_length_prefix` as a no-op to save cycles, as our keys are implicitly
   length-bounded.

### Map Capacity & Cache Misses

rust
const MAX_STATION_CAPACITY: usize = 1_000;
*Why not pre-allocate 100,000 to be safe?*
Because of **CPU Cache constraints**. The dataset has exactly 413 unique stations. If we allocate space for 100,000
stations, the `HashMap` underlying memory buffer becomes massive and *sparse*.
When the CPU looks up a station, it pulls a cache line from RAM. If the table is sparse, that cache line is mostly
empty, leading to massive cache misses and wasted memory bandwidth. By clamping the capacity to `1_000`, the hash table
stays extremely dense, fitting snugly into the CPU's ultra-fast L1/L2 cache.

---

## 6. Aggregation & Output Formatting

The values stored in the map are: `min`, `sum`, `count`, `max`.

* **Memory Layout:** We ensure `sum` is an `i64` to prevent overflow over a billion rows, while `min` and `max` remain
  `i16`.
* **Deferred Math:** We *never* calculate the mean in the hot loop. Floating-point division is one of the most expensive
  CPU instructions. We only divide at the very end.
* **Sorting (`BTreeMap`):** The output requires alphabetical sorting. We take the merged `HashMap`, convert the raw
  `&[u8]` keys to `String` (using `from_utf8_unchecked` since the README guarantees valid UTF-8), and collect them into
  a `BTreeMap` which inherently maintains a sorted order.

---

## Summary of Optimizations

| System/Concept  | Standard/Idiomatic Rust    | Our Systems Approach             | Primary Benefit                              |
|:----------------|:---------------------------|:---------------------------------|:---------------------------------------------|
| **I/O**         | `File::open` + `BufReader` | `mmap` + `madvise`               | Zero memory copies, OS-level read-ahead.     |
| **Concurrency** | Single-threaded            | Byte-chunking + `thread::scope`  | $O(N)$ speedup with zero mutex contention.   |
| **Scanning**    | `.split(b'\n')` (Iterator) | `u8x64` SIMD + `libc::memchr`    | Checks 64 bytes per cycle; no bounds checks. |
| **Parsing**     | `f64::parse()`             | Branchless ASCII math            | Avoids FPU overhead and pipeline flushes.    |
| **Hashing**     | `SipHash`                  | Custom XOR + Rotate `DumbHasher` | Drastically faster map insertions/lookups.   |
| **Memory**      | Dynamic `HashMap` growth   | Dense `1_000` pre-allocation     | Maximizes L1/L2 CPU Cache locality.          |

### FAQ

1. **Why does `next_newline` use raw pointers (`unsafe { slice.get_unchecked }`)?**
   In Go or standard Rust, accessing `slice[i]` includes a hidden `if i < slice.len() { panic }`. Inside a loop running
   1 billion times, that `if` statement adds up. By using raw pointers and `get_unchecked`, you are taking
   responsibility for the bounds. You are telling the compiler: "I guarantee this is safe, do not emit the `if` check
   assembly." This allows the CPU to keep those memory addresses directly in its fastest registers without branching.

2. **Branchless Code & The Pipeline:**
   Your temperature parser is brilliant. A CPU executes instructions like an assembly line (fetch, decode, execute,
   write-back). If it hits an `if` statement (a branch), it has to guess which path to take to keep the assembly line
   moving. If it guesses wrong, it throws away all the work on the assembly line (a pipeline flush).
   By doing `let sign = i16::from(is_positive) * 2 - 1;`, you do math *instead* of logic. Math never causes a pipeline
   flush.

3. **Cache Locality (The `1000` vs `100_000` HashMap Capacity):**
   RAM is actually incredibly slow compared to the CPU core. To compensate, CPUs have L1, L2, and L3 caches right on the
   chip. When you ask for a byte of memory, the CPU actually grabs a whole 64-byte chunk (a Cache Line) from RAM and
   puts it in L1.
   If your HashMap has a capacity of 100,000 but only 413 entries, the entries are scattered miles apart in memory.
   Every lookup requires a slow trip to RAM. If the capacity is 1,000, all 413 entries are packed tightly together. When
   the CPU fetches one station, it accidentally fetches 3 or 4 other stations into the cache for free!
