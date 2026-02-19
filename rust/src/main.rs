#![feature(portable_simd)]
#![feature(slice_split_once)]
#![feature(hasher_prefixfree_extras)]

use crate::hasher::DumbHasherBuilder;
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    os::fd::AsRawFd,
    simd::{cmp::SimdPartialEq, u8x64},
    {env, io, os, ptr, slice},
};

#[cfg(test)]
mod tests;

mod hasher;

mod ascii {
    pub const MINUS: u8 = b'-';
    pub const ZERO: u8 = b'0';
}

mod layout {
    pub const TWO_DIGIT_FORMAT_LEN: usize = 4;

    pub const MULTIPLIER_TWO_DIGIT: i16 = 100;
    pub const MULTIPLIER_ONE_DIGIT: i16 = 10;
}

mod bytes {
    pub const FIRST: usize = 0;
    pub const SECOND: usize = 1;
}

const DEFAULT_FILE_PATH: &str = "../measurements.txt";
const DELIMITER_SEMI: u8x64 = u8x64::splat(b';'); // 64 u8's -> [';', ';', ... ';'] 0..63
const DELIMITER_NEW_L: u8x64 = u8x64::splat(b'\n'); // 64 u8's -> ['\n', \n', ... '\n'] 0..63

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        args[1].as_str()
    } else {
        DEFAULT_FILE_PATH
    };
    let stats = process_file(file_path);
    let output = format_output(stats);
    println!("{output}");
    println!();
}

// -------------------------------------------- Helper Functions --------------------------------------------

/// Processes the input file and computes per-station statistics.
///
/// # Performance notes
/// This function is optimized for large, sequential scans:
///
/// - The file is memory-mapped to avoid buffered I/O overhead.
/// - Lines are located using `memchr`, which performs a raw byte search
///   instead of iterator-based splitting.
/// - Field separation avoids `split` and closures to reduce per-byte
///   overhead in the hot loop.
///
/// The input format is assumed to be ASCII with lines of the form:
/// `station_name;temperature\n`.
///
/// # Safety
/// Uses `libc::memchr` on memory-mapped data. The pointers passed to
/// `memchr` are guaranteed to be valid for the provided length.
fn process_file(file_path: &str) -> HashMap<Vec<u8>, (i16, i64, usize, i16), DumbHasherBuilder> {
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Could not open {} file", file_path));

    //note: README promised 413 weather stations; 1000 gives headroom without over-allocating
    // Why 1000? The dataset has exactly 413 unique station names. Pre-allocating for 100,000
    // caused massive over-allocation, leading to:
    // - Sparse hash table → more cache misses
    // - Wasted memory bandwidth
    // - Poorer CPU cache utilization
    // 1000 gives us ~2.4x headroom (enough to avoid reallocation) while keeping the table dense.
    const MAX_STATION_CAPACITY: usize = 1_000;
    let mut stats = HashMap::<Vec<u8>, (i16, i64, usize, i16), _>::with_capacity_and_hasher(
        MAX_STATION_CAPACITY,
        hasher::DumbHasherBuilder,
    );
    let mut at = 0;
    //note: We know we're going to read the whole file, so buffered reading isn't optimal.
    // Memory mapping tells the kernel to make the file accessible as memory.
    let mmap = mmap_file(&file);

    //note: Changed from loop with next_line to while loop with next_newline
    // This allows us to use SIMD for newline detection instead of just memchr
    while at < mmap.len() {
        // let line = next_line(mmap, &mut at);
        let newline_at = at + next_newline(mmap, at);
        let line = &mmap[at..newline_at];
        at = newline_at + 1; //note: current line processing finished - go to next line

        // if line.is_empty() {
        //     break;
        // }

        let (station, temperature) = split_semi(line);
        process_line((station, temperature), &mut stats);
    }

    // mmap is automatically unmapped when it goes out of scope (see mmap_file docs)
    stats
}

/// Memory-map a file into read-only byte slice using `libc::mmap`.
///
/// This function creates a read-only memory mapping of the entire file,
/// allowing direct byte access without copying data into userspace buffers.
/// The mapping is backed by the file on disk and shares memory with other
/// processes mapping the same file (`MAP_SHARED`).
///
/// # Performance Characteristics
/// - **Zero-copy**: Data is accessed directly from kernel page cache
/// - **Lazy loading**: Pages are loaded on-demand (demand paging)
/// - **Efficient random access**: Constant-time O(1) access to any byte offset
/// - **Kernel-managed caching**: OS handles page cache automatically
///
/// # Safety
/// - The returned slice is valid while the mapping exists i.e., until the file is closed.
/// - **IMPORTANT**: The slice lifetime is tied to the underlying mapping,
///   not the `File` parameter. This function's signature is misleading.
/// - The caller must ensure the file is not mutated while mapped (undefined behavior)
/// - The mapping is automatically unmapped when the slice goes out of scope
///   (via the OS when process exits, but Rust doesn't track this lifetime)
///
/// # Panics
/// - If file metadata cannot be read
/// - If `mmap` system call fails (e.g., insufficient memory, invalid file descriptor)
///
/// A byte slice (`&[u8]`) referencing the memory-mapped file contents.
/// **WARNING**: The actual lifetime is not encoded in Rust's type system.
fn mmap_file(file: &File) -> &[u8] {
    let len = file.metadata().expect("Could not read metadata").len();

    // SAFETY: libc usage
    unsafe {
        const OFFSET: libc::off_t = 0;
        let ptr = libc::mmap(
            ptr::null_mut(),     // Let OS choose address (you don't care where)
            len as libc::size_t, // Len of file - How many bytes to map
            libc::PROT_READ,     // Memory protection: read-only
            libc::MAP_SHARED,    // Changes visible to other processes & persisted to file
            file.as_raw_fd(),    // File descriptor to map
            OFFSET, // Offset of where we want to read from - Start mapping from beginning of file
        );

        if ptr == libc::MAP_FAILED {
            panic!(
                "failed to map file to mmap: {:?}",
                io::Error::last_os_error()
            )
        }

        //note: advise os on how this memory map will be accessed.
        // We're telling the kernel that when we read from a byte
        // offset, we're going to be reading in a sequential order,
        // so feel free to read ahead more (huge ass more) in advance.
        if libc::madvise(ptr, len as usize, libc::MADV_SEQUENTIAL) != 0 {
            panic!(
                "failed to advise os on how this memory map will be accessed: {:?}",
                io::Error::last_os_error()
            )
        }

        let data = ptr as *const u8;
        let number_of_elements = len as usize;
        slice::from_raw_parts(data, number_of_elements)
    }
}

/// Returns the next line from the memory-mapped file.
///
/// # Arguments
/// * `mmap` - The memory-mapped file contents
/// * `at` - Current position in the file (mutated to point to next line)
///
/// # Returns
/// A byte slice containing the next line (without newline)
///
/// # Note
/// When EOF is reached, returns the remaining data. The next call will
/// find an empty slice and the loop should break.
#[allow(dead_code)]
#[deprecated]
fn next_line<'a>(mmap: &'a [u8], at: &mut usize) -> &'a [u8] {
    let remaining_mmap_data = &mmap[*at..];
    //note: memchr returns a pointer to where that char appears.
    // ~cppreference website:
    //      Pointer to the location of the byte, or a null pointer if no such byte is found.
    //SAFETY: remaining_mmap_data is valid for at least remaining_mmap_data.len() bytes,
    // which is exactly the range we're searching within.
    let next_newline = unsafe {
        libc::memchr(
            remaining_mmap_data.as_ptr() as *const os::raw::c_void,
            b'\n' as os::raw::c_int,
            remaining_mmap_data.len(),
        )
    };
    let newline_ptr = unsafe {
        libc::memchr(
            remaining_mmap_data.as_ptr() as *const libc::c_void,
            b'\n' as libc::c_int,
            remaining_mmap_data.len(),
        )
    };
    let line = if newline_ptr.is_null() {
        //note: No newline found - we're at EOF. Return everything remaining.
        // The next call will find at == mmap.len() and return empty slice.
        // There's no need to remember to break on new line
        // since next iteration will find empty line.
        // We're basically saying:
        // if we don't find \n character, line is from at -> EOF

        remaining_mmap_data
    } else {
        //note: Otherwise;
        // - `next_newline` is a `*const c_void` pointer to where '\n' was found
        // - `remaining_mmap_data.as_ptr()` is pointer to start of our slice
        // - `.offset_from()` returns the signed distance between two pointers (in bytes)
        // - Since `next_newline` is always ≥ `remaining_mmap_data.as_ptr()`, the result is positive
        // - We cast to usize to get the length

        let next_newline = next_newline as *const u8;

        //SAFETY: memchr always returns pointer in remaining_mmap_data bounds,
        // which are valid so offset_from gives us the exact line length.
        let len = unsafe { next_newline.offset_from(remaining_mmap_data.as_ptr()) };

        //note: ~Jon Gjengset:
        //          we happen to know that next_newline is always greater than
        //          the pointer we pass in and as such we know this is positive.
        let len = len as usize;
        &remaining_mmap_data[..len]
    };
    *at += line.len() + 1; //note: +1 to skip the newline character; skipping over the line we found + newline

    line
}

/// Finds the position of the next newline character using SIMD operations.
///
/// This function searches for a newline character starting from position `at` in the memory map.
/// It uses SIMD instructions to check 64 bytes at once for efficiency, falling back to `memchr`
/// if the newline is not found in the first 64 bytes.
///
/// # Arguments
/// * `mmap` - The memory-mapped file contents
/// * `at` - Current position in the file to start searching from
///
/// # Returns
/// The relative offset from the starting position `at` where the newline character was found
///
/// # Note
/// This function assumes there is always a newline character in the remaining data
/// (which is true for the 1BRC input format).
fn next_newline(mmap: &[u8], at: usize) -> usize {
    let remaining = &mmap[at..];

    //note: Use SIMD to check the first 64 bytes for newline character in parallel
    let newline_eq = DELIMITER_NEW_L.simd_eq(u8x64::load_or_default(remaining));
    if let Some(new_l_pos) = newline_eq.first_set() {
        new_l_pos
    } else {
        //note: If SIMD didn't find a newline in the first 64 bytes, fall back to memchr
        // We know line is at most 106 bytes (100 chars + semicolon + 5 digits), but we can only search 64 bytes
        // So the search may have to fall back to memchr if newline is beyond first 64 bytes
        // We know there must be a newline, so rest[64..] must be non-empty
        let rest_remaining = &remaining[64..];
        // SAFETY: rest_remaining is valid for at least rest_remaining.len() bytes
        let next_newline = unsafe {
            libc::memchr(
                rest_remaining.as_ptr() as *const os::raw::c_void,
                b'\n' as os::raw::c_int,
                rest_remaining.len(),
            )
        };

        //note: Assert that we found a newline since the input format guarantees it
        assert!(!next_newline.is_null());

        // SAFETY: memchr always returns pointers in rest_remaining, which are valid
        let len =
            unsafe { (next_newline as *const u8).offset_from(rest_remaining.as_ptr()) } as usize;
        64 + len
    }
}

fn split_semi(line: &[u8]) -> (&[u8], &[u8]) {
    //note: We know, line is at most 100 character for station names (100),
    // plus semicolon (1), plus at most two digits decimal points fraction (5)
    // 100 + 1 + 5 = 106 bytes.
    if line.len() > 64 {
        //note: Slow path
        // In case the station is a very long station (more than 64 bytes
        // which basically won't happen)
        let (station, temperature) = line
            .rsplit_once(|char| *char == b';')
            .expect("failed to extract temperature from split-ted fields");
        (station, temperature)
    } else {
        //note: Fast path
        // Get the semicolon and current line as simd things,
        // and do concurrent/parallel equality check between these two.
        let delim_eq_mask = DELIMITER_SEMI.simd_eq(u8x64::load_or_default(line));
        // SAFETY: 1BRC README promised every line has delimiter i.e., ';'
        let index_of_delim = unsafe { delim_eq_mask.first_set().unwrap_unchecked() };

        (&line[..index_of_delim], &line[index_of_delim + 1..])
    }
}

/// Processes a single line and updates the stats map.
fn process_line(
    line: (&[u8], &[u8]),
    stats: &mut HashMap<Vec<u8>, (i16, i64, usize, i16), DumbHasherBuilder>,
) {
    let (station, temperature) = line; // avoid utf-8 parsing except for temperature
    let temperature = parse_temperature(temperature);

    // Get or insert default value for the station
    let entry = match stats.get_mut(station) {
        Some(existing_stats) => existing_stats,
        None => stats
            .entry(station.to_vec())
            .or_insert((i16::MAX, 0, 0usize, i16::MIN)),
    };

    // Update the min, sum, count, and max values for the station
    entry.0 = entry.0.min(temperature); // min
    entry.1 += i64::from(temperature); // running sum
    entry.2 += 1; // count
    entry.3 = entry.3.max(temperature); // max
}

/// Parses a temperature value encoded as ASCII bytes into a fixed-point integer.
///
/// # Format
/// The input must be in the form:
/// - `"12.3"`
/// - `"-4.7"`
/// - `"0.0"`
///
/// The value is returned as **tenths of a degree**:
/// - `"12.3"`  → `123`
/// - `"-4.7"`  → `-47`
///
/// # Notes
/// - This function performs **no UTF-8 validation**
/// - No floating-point operations are used
/// - Assumes exactly **one decimal digit**
/// - Designed for high-performance parsing in hot loops (1BRC-style)
#[inline(never)]
fn parse_temperature(temperature: &[u8]) -> i16 {
    let tlen = temperature.len();
    assert!(tlen >= 3); //note: the input is at least 3 bytes long (the shortest possible is “0.0”).

    let first_byte_is_minus = temperature[bytes::FIRST] == ascii::MINUS;

    //note: Branchless Sign Calculation:
    // This single line calculates the sign (1 or -1) using pure arithmetic, with no if statement and thus no potential for a branch misprediction.
    // Case 1: Positive number (e.g., temperature[0] is b'9').
    //  - temperature[0] != b'-' evaluates to true.
    //  - i16::from(true) converts the boolean true to the integer 1.
    //  - The calculation becomes 1 * 2 - 1, which equals 1.
    // Case 2: Negative number (e.g., temperature[0] is b'-').
    //  - temperature[0] != b'-' evaluates to false.
    //  - i16::from(false) converts the boolean false to the integer 0.
    //  - The calculation becomes 0 * 2 - 1, which equals -1.
    let is_positive = !first_byte_is_minus;
    let sign = i16::from(is_positive) * 2 - 1; // or let sign = 1 - ((first_byte_is_minus as i16) << 1); ==> [minus → 1 - 2 = -1, positive → 1 - 0 = 1]

    //note: Fake Branches:
    // These look exactly like branches. However, a modern optimizing compiler (like Rust’s) is extremely smart.
    // For simple if/else statements that just select a value, it can often translate them into a conditional move instruction (e.g., cmov on x86).
    // A cmov instruction doesn’t cause a pipeline-flushing jump. It effectively says: “I’ve already calculated both values;
    // now, based on this flag, move either value A or value B into the destination register.” It’s a conditional "operation", not a conditional "jump",
    // which is much faster if the prediction is hard.
    //
    //note: determines the starting position for parsing the digits. If there’s a negative sign, we need to skip the first byte.
    // The compiler will likely turn this into a cmov.
    let sign_offset = if first_byte_is_minus { 1 } else { 0 };
    //note: trick to figure out if we’re parsing a one-digit or two-digit number (before the decimal).
    // * tlen - sign_offset gives the length of the number part
    //   (e.g., for "-98.2", tlen is 5, sign_offset is 1, so 5-1=4. For "9.2", tlen is 3, sign_offset is 0, so 3-0=3).
    // * If the number part has 4 characters (e.g., “98.2”), it must be a two-digit number. The first digit '9' represents 90, which is 9 * 10. To get 982,
    //   we need to treat the first digit as hundreds. So multiplier becomes 100.
    // * If the number part has 3 characters (e.g., “9.2”), it’s a one-digit number. The first digit '9' represents 90, which is 9 * 10. So multiplier becomes 10.
    let number_len = tlen - sign_offset;
    let multiplier = if number_len == layout::TWO_DIGIT_FORMAT_LEN {
        layout::MULTIPLIER_TWO_DIGIT
    } else {
        layout::MULTIPLIER_ONE_DIGIT
    };

    //note: Parsing the Digits:
    // Parse the number as a sum of three parts: t1, t2, and t3, which represent the hundreds, tens, and units digits of the final integer value.
    let first_digit_byte = temperature[sign_offset]; // Gets the first digit of the number, whether there was a sign or not.
    let first_digit = first_digit_byte - ascii::ZERO; // Standard ASCII trick to convert a character digit ('0' to '9') into its integer value (0 to 9).
    //note: The first digit its correct magnitude - always exists.
    // For “98.2”, multiplier is 100, first_digit_byte is b'9'. t1 becomes 100 * 9 = 900.
    // For “9.2”, multiplier is 10, first_digit_byte is b'9'. t1 becomes 10 * 9 = 90.
    let t1 = multiplier * i16::from(first_digit);
    //note: Calculates the value of the “tens” digit, but only if it exists.
    // has_second_digit: This is another branchless switch.
    //   * If it’s a one-digit number (“9.2”), multiplier is 10. This expression becomes 0. The entire t2 calculation becomes 0, effectively ignoring the tens digit.
    //   * If it’s a two-digit number (“98.2”), multiplier is 100. This expression becomes 1. The t2 calculation proceeds.
    // temperature[tlen - 3]: This index always points to the second digit of a two-digit number.
    //   * For “98.2” (tlen=4), temperature[4-3] is temperature[1], which is b'8'.
    //   * For “-98.2” (tlen=5), temperature[5-3] is temperature[2], which is b'8'.
    //   So for “98.2”, t2 becomes 1 * 10 * 8 = 80. For “9.2”, it becomes 0.
    let has_second_digit = if multiplier == 10 { 0 } else { 1 };
    let second_digit_of_two_digit_number = temperature[tlen - 3];
    let t2 = has_second_digit * 10 * i16::from(second_digit_of_two_digit_number - ascii::ZERO);
    //note: ...
    // temperature[tlen - 1]: This always points to the very last byte, which is the digit after the decimal point (the “units” digit of our final integer).
    // For “98.2”, this is b'2', so t3 is 2.
    // For “9.2”, this is b'2', so t3 is 2.
    let units_digit = temperature[tlen - bytes::SECOND];
    let t3 = i16::from(units_digit - ascii::ZERO);

    //note: Finally, the three parts are summed and multiplied by the sign we calculated at the very beginning.
    sign * (t1 + t2 + t3)
}

/// Formats the statistics into the required output format.
fn format_output(stats: HashMap<Vec<u8>, (i16, i64, usize, i16), DumbHasherBuilder>) -> String {
    // We can;
    // a) sort all the keys,
    // b) move them into BTreeMap
    // we'll go with a
    let mut output = String::from("{");
    let stats = BTreeMap::from_iter(
        stats
            .into_iter()
            // SAFETY: 1BRC README.md promised valid utf-8 string characters
            .map(|(k, v)| (unsafe { String::from_utf8_unchecked(k) }, v)),
    );
    let mut stats = stats.iter().peekable();

    while let Some((station, (min, sum, count, max))) = stats.next() {
        output.push_str(&format!(
            "{station}={min:.1}/{mean:.1}/{max:.1}",
            station = station,
            min = (*min as f64) / 10_f64,
            mean = (*sum as f64) / 10_f64 / (*count as f64),
            max = (*max as f64) / 10_f64
        ));

        // Add comma separator if there are more items to come
        if stats.peek().is_some() {
            output.push_str(", ");
        }
    }

    output.push('}');
    output
}
