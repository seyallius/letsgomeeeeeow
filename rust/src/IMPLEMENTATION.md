# 1BRC Implementation Deep Dive

This document explains the implementation details of the One Billion Row Challenge (1BRC) solution in Rust. This
solution is heavily optimized for performance, moving away from idiomatic, safe Rust into "systems programming"
territory using direct memory management, SIMD instructions, and custom hashing.

## High-Level Architecture

The flow of the program is linear:

1. **Memory Map** the input file (treat the hard drive file as if it were RAM).
2. **Iterate** through the file byte-by-byte (mostly) to find newlines.
3. **Parse** each line to separate the station name from the measurement.
4. **Aggregate** data (min, max, sum, count) into a Hash Map.
5. **Sort and Print** the results.

---

## 1. The Entry Point: `process_file`

The standard way to read a file in Rust is `BufReader`. However, `BufReader` involves copying data from the kernel's
file cache into a user-space buffer, and then iterating over it.

### Memory Mapping (`mmap`)

Instead, we use `mmap` (Memory Map).

```rust
let mmap = mmap_file( & file);
```

* **What is it?** It tells the Operating System: "Take the file descriptor for `measurements.txt` and map its contents
  directly into my process's virtual memory address space."
* **Why is it faster?** It is **Zero-Copy**. When we read from the `mmap` slice, we are reading directly from the OS
  page cache. We skip the overhead of copying bytes into a Rust `Vec` or `String`.
* **MADV_SEQUENTIAL:** Inside `mmap_file`, we call `libc::madvise`. This tells the Linux kernel: "I am going to read
  this file from start to finish linearly. Please aggressively pre-load (read-ahead) data from the disk into RAM before
  I even ask for it."

### The HashMap Setup

```rust
let mut stats = HashMap::with_capacity_and_hasher(
  MAX_STATION_CAPACITY,
  hasher::DumbHasherBuilder,
);
```

Two critical optimizations happen here:

1. **Capacity (`100_000`):** If we don't set this, the map starts small. As we add stations, the map fills up, and Rust
   has to pause, allocate a bigger chunk of memory, and copy everything over (reallocating). By setting it high
   initially, we allocate once and never resize.
2. **`DumbHasher`:** (Explained in detail in Section 5).

---

## 2. Reading Lines: `next_line` & `memchr`

We need to find where a line ends (`\n`). A standard iterator like `.split(b'\n')` is safe but checks bounds on every
byte.

### `libc::memchr`

```rust
let next_newline = unsafe {
  libc::memchr(..., b'\n', ...)
};
```

* **What is it?** This is a C library function. It is heavily optimized assembly code. It doesn't look at bytes one by
  one; it loads a whole "word" (64 bits or more) of data and checks for the newline character in parallel using CPU
  tricks.
* **Pointer Arithmetic:** Once `memchr` finds the address of the newline, we calculate the length of the line by
  subtracting pointers: `address_of_newline - address_of_start`.
* **Safety:** This is `unsafe` because we are dealing with raw pointers. We must guarantee that we don't look past the
  end of the memory map.

---

## 3. Splitting the Station and Temperature: SIMD

Once we have a line (e.g., `Hamburg;12.3`), we need to find the semicolon `;`.

### The Naive Way vs. SIMD

The naive way is to loop `for char in line { if char == ';' ... }`.
We use **SIMD (Single Instruction, Multiple Data)**.

```rust
const DELIMITER_SEMI: u8x64 = u8x64::splat(b';');
// ...
let delim_eq_mask = DELIMITER_SEMI.simd_eq(u8x64::load_or_default(line));
```

1. **`u8x64`:** This represents a vector of 64 bytes fitting into a special CPU register (AVX-512 if available, or
   fallback).
2. **`splat`:** We create a register filled entirely with semicolons: `[;, ;, ;, ... ;]`.
3. **`load_or_default`:** We load the first 64 bytes of the line into another register.
4. **`simd_eq`:** The CPU compares all 64 bytes **simultaneously** in a single clock cycle (roughly).
5. **`first_set()`:** This returns the index of the first match (the semicolon).

**Why fallback logic?**
If the line is massive (over 64 bytes), our fixed-size SIMD approach gets complex. Since we know station names are
usually short, we use the super-fast SIMD path for lines < 64 bytes. If a station name is huge (rare), we fall back to
standard Rust `.rsplit_once()`.

---

## 4. Integer Parsing: `parse_temperature`

We encounter numbers like `12.3`, `-5.0`, `9.8`.
Parsing these as `f64` (floats) is slow because floating-point math standards (IEEE 754) are complex.

**The Fixed-Point Trick:**
We ignore the dot.

* `12.3` becomes `123` (integer).
* `-5.0` becomes `-50` (integer).
* We store everything as `i16` (tenths of a degree).

**Manual ASCII Conversion:**
Instead of `String::parse()`, which handles Unicode and errors, we do raw math:
rust
parsed += i16::from(digit - b'0') * place;
If the character is `'3'` (ASCII value 51), subtracting `'0'` (ASCII value 48) gives the integer `3`. This is raw CPU
subtraction, much faster than a parser state machine.

---

## 5. The `DumbHasher`

Rust's default `HashMap` uses **SipHash**. SipHash is "cryptographically strong," meaning it is designed to prevent
HashDoS attacks (where a hacker sends specific keys to make your hashmap slow). SipHash is high-quality but slow.

In 1BRC, we trust the input. We don't need security; we need speed.

### Polynomial Rolling Hash

```rust
let mixed = self .0 as u128 * (u64::from_ne_bytes(chunk) as u128);
self .0 = (mixed > > 64) as u64 ^ mixed as u64;
```

1. **Chunks:** We read the station name in 8-byte chunks (`u64`).
2. **Math:** We multiply the chunk by the current hash state and XOR the results.
3. **Why "Dumb"?** It doesn't handle collisions as perfectly as SipHash, and it's predictable. But it drastically
   reduces the CPU cycles needed to look up a station in the map.

---

## 6. Aggregation Logic

The values stored in the map are:
`min (i16)`, `sum (i64)`, `count (usize)`, `max (i16)`.

* **Min/Max:** Standard `entry.min(new_val)`.
* **Sum:** We use `i64` to ensure the sum never overflows, even with a billion rows.
* **Count:** Simply incremented.

We do *not* calculate the mean here. Division is expensive. We only calculate the mean at the very end when printing.

---

## 7. Output Formatting

rust
let stats = BTreeMap::from_iter(...)
Standard `HashMap` order is random (and depends on the hash). The challenge requires output sorted alphabetically by
station name.

* We convert the `HashMap` into a `BTreeMap`. A `BTreeMap` automatically keeps its keys sorted.
* We convert our fixed-point integers back to floats only at the print step:
* `min`: `val as f64 / 10.0`
* `mean`: `sum as f64 / 10.0 / count as f64`

## Summary of Optimizations

| Optimization    | Standard Rust               | Our Approach                 | Benefit                          |
|:----------------|:----------------------------|:-----------------------------|:---------------------------------|
| **I/O**         | `BufReader` (buffered copy) | `mmap` (kernel page cache)   | Zero memory copying.             |
| **Scanning**    | Iterator `.next()`          | `memchr` & Pointers          | CPU vectorization for newlines.  |
| **Parsing**     | `str::split`                | SIMD (`u8x64`)               | Finds `;` in parallel execution. |
| **Numbers**     | `f64::parse`                | Manual ASCII math (Integers) | Avoids FPU and parsing logic.    |
| **Hashing**     | `SipHash` (Secure)          | `DumbHasher` (Simple math)   | Faster Map lookups/inserts.      |
| **Allocations** | Dynamic growing             | Pre-allocated capacity       | No memory reallocation pauses.   |
