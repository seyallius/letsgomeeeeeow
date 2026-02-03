# 1BRC Implementation Deep Dive (Go)

This document explains the implementation details of the current Go solution for the One Billion Row Challenge (1BRC).

While this version is currently "behind" the Rust version in terms of micro-optimizations (like SIMD or custom hashers),
it establishes the foundational architecture by moving away from standard Buffered I/O to direct System Calls.

## High-Level Architecture

The flow of the program is:

1. **Memory Map** the file using low-level system calls.
2. **Iterate** over the raw byte slice to find lines.
3. **Convert** bytes to strings (standard Go handling).
4. **Parse** using standard library functions (`strconv`, `strings`).
5. **Aggregate** using the built-in Go `map`.

---

## 1. The Entry Point: `processFile`

In standard Go, you would typically use `os.Open` followed by `bufio.NewScanner` to read a file line-by-line. While
idiomatic, `bufio` involves copying data from the kernel into a Go buffer, and then potentially copying it again into
strings.

### The `mmap` Approach

Instead, we use **Memory Mapping**.

```go
mmap := mmapFile(file)
defer syscall.Munmap(mmap)
```

This maps the file directly into the process's memory space. To the Go runtime, the entire 1GB+ file looks like a giant
`[]byte` slice. This avoids the overhead of function calls like `Read()` inside a loop.

---

## 2. Low-Level Memory Mapping: `mmapFile`

Go's standard library does not expose `mmap` directly in high-level packages (like `os` or `io`). We must access the
operating system directly via the `syscall` package.

### `syscall.Mmap`

```go
data, err := syscall.Mmap(
    int(file.Fd()),     // File Descriptor
    0,                  // Offset
    fileSize,           // Length
    syscall.PROT_READ,  // Read-only protection
    syscall.MAP_SHARED, // Shared mapping
)
```

* **Zero-Copy:** We are asking the OS to give us a pointer to the file data in the page cache. We don't copy bytes into
  a user-space buffer.
* **Safety:** This is technically safe in Go (it returns a `[]byte`), but if the file changes underneath us, behavior
  can be unpredictable.

### `syscall.Madvise`

```go
syscall.Madvise(data, syscall.MADV_SEQUENTIAL)
```

* **The Hint:** We explicitly tell the Linux Kernel: "We are going to read this huge slice from index 0 to the end, in
  order."
* **The Result:** The OS detects this pattern and triggers "Read-Ahead." It loads chunks of the file from the SSD/HDD
  into RAM *before* our code actually iterates over index `i`. This minimizes CPU wait time.

---

## 3. The Parsing Loop

We iterate over the raw memory map using a simple range loop:

```go
start := 0
for i, b := range mmap {
    if b == '\n' {
        // ... process line ...
    }
}
```

### The Allocation Bottleneck (Current State)

Currently, inside the loop, we do this:
go
line := string(mmap[start:i])

* **What happens here:** In Go, casting `[]byte` to `string` creates a **copy** of the data and allocates memory on the
  Heap.
* **Why it works:** It's safe and allows us to use standard string functions.
* **Performance Note:** In a high-optimized version (like the Rust one), we would avoid this allocation and work purely
  with `[]byte`. For now, this is the easiest way to make the code work.

---

## 4. Line Parsing: `processLine`

Once we have a `line` string (e.g., `"Hamburg;12.3"`), we split it.

### Finding the Delimiter

```go
lastSemicolon := strings.LastIndex(line, ";")
```

* **Why `LastIndex`?** We scan from the right side. The temperature is usually short (3 to 5 chars), while the station
  name can be long. Scanning from the right is statistically faster to find the separator.

### Parsing Floats

```go
temperature, err := strconv.ParseFloat(temperatureStr, 64)
```

* **Standard Library:** We currently use Go's built-in float parser. This handles all edge cases (like scientific
  notation, though not needed here) but carries overhead compared to raw integer math.

---

## 5. Aggregation: The Map

```go
stats := make(map[string][4]float64)
```

### The Data Structure

We store a fixed-size array of 4 floats for every station: `[min, sum, count, max]`.

### Map Access Pattern

```go
tup, exists := stats[station]
// ... update tup ...
stats[station] = tup
```

* **Value Types:** In Go, arrays (`[4]float64`) are **values**, not references. When we do `tup := stats[station]`, we
  get a *copy* of the array.
* **Write Back:** After modifying `tup` (adding to sum, updating max), we **must** write it back to the map:
  `stats[station] = tup`.
* **Initialization:** If the station doesn't exist, we manually initialize `Min` to a very large number and `Max` to a
  very small number to ensure the first comparison works correctly.

---

## 6. Output Formatting

```go
var output strings.Builder
```

* **`strings.Builder`:** In Go, strings are immutable. Using `s += "next part"` creates garbage string objects.
  `strings.Builder` is the efficient way to concatenate text; it writes to an internal buffer and only creates the final
  string once.
* **Sorting:** Go maps are unordered. We extract all keys into a slice, call `sort.Strings()`, and then iterate through
  that sorted slice to print in alphabetical order.

---

## Summary of Current Implementation

| Feature             | Implementation              | Performance Impact                                              |
|:--------------------|:----------------------------|:----------------------------------------------------------------|
| **File Reading**    | `syscall.Mmap`              | **Fast**. Zero-copy access to disk data.                        |
| **Iteration**       | Byte-by-byte range loop     | **Fast**. Simple linear scan.                                   |
| **String Handling** | `string([]byte)` conversion | **Slow**. Allocates memory for every single line.               |
| **Parsing**         | `strconv.ParseFloat`        | **Medium**. accurate but slower than integer math.              |
| **Storage**         | Native Go Map               | **Medium**. Good general performance, but has hashing overhead. |

**Next Steps for Optimization:**
To catch up to the Rust implementation, the next steps for this Go code would be:

1. Stop converting `[]byte` to `string` (avoid heap allocation).
2. Use a custom hash table or open-addressing map instead of the native Go map.
3. Parse temperatures as integers manually (ignoring the decimal point).
