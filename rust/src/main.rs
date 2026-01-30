use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::{env, io, ptr, slice};

#[cfg(test)]
mod tests;

// -------------------------------------------- Types, Variables & Consts --------------------------------------------

const DEFAULT_FILE_PATH: &str = "../measurements.txt";

/// Holds station statistics together with the backing memory map.
///
/// # Why does this exist?
/// The HashMap keys are `&[u8]` slices that point directly into a
/// memory-mapped file. In Rust, references must never outlive the data
/// they point to.
///
/// This struct ensures:
/// - The memory map (`_mmap`) stays alive
/// - All keys in `map` remain valid
///
/// # Performance
/// - Zero allocations
/// - Zero copying
/// - Zero runtime cost
///
/// This is purely a *lifetime anchor* for soundness.
struct Stats<'a> {
    /// Memory-mapped file backing all station name slices.
    ///
    /// This field is intentionally unused. Its sole purpose is to
    /// keep the mmap alive for as long as `map` exists.
    _mmap: &'a [u8],

    /// Station statistics keyed by station name slices.
    statistics: HashMap<&'a [u8], (f64, f64, usize, f64)>,
}

// -------------------------------------------- Main --------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_path = if args.len() > 1 {
        args[1].as_str()
    } else {
        DEFAULT_FILE_PATH
    };
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Could not open {} file", file_path));
    let stats = process_file(&file);
    let output = format_output(stats.statistics);
    println!("{output}");
    println!();
}

// -------------------------------------------- Helper Functions --------------------------------------------

/// Processes a file and returns aggregated statistics for all stations.
///
/// # Lifetimes
/// The returned `Stats<'a>` borrows directly from a memory-mapped view
/// of `file`. All station name keys inside the returned map are slices
/// pointing into that mapping.
///
/// The lifetime `'_` ensures the memory map remains valid for as long
/// as the statistics are used.
///
/// # Design Notes
/// - Station names are kept as `&[u8]` to avoid UTF‑8 validation and
///   allocation during parsing.
///
/// # Safety
/// This function relies on `mmap_file`, whose signature does not encode
/// the true lifetime of the mapping. Correctness is ensured by storing
/// the returned slice inside `Stats`, preventing it from escaping.
fn process_file(file: &File) -> Stats<'_> {
    //note: The key is slice of u8 bytes as we already have the data in mmap,
    // there isn't really needed to parse the keys into strings.
    // ~Jon Gjengset:
    //      because it can be references into the mmap,
    //      there's nothing that needs to be owned about.
    let mut stats = HashMap::<&[u8], (f64, f64, usize, f64)>::new();

    //note: We know we're going to read the whole file, so buffered reading isn't optimal.
    // Memory mapping tells the kernel to make the file accessible as memory.
    let mmap = mmap_file(file);

    for line in mmap.split(|char| *char == b'\n') {
        if line.is_empty() {
            break;
        }

        let mut fields = line.rsplitn(2, |char| *char == b';');
        let temperature = fields
            .next()
            .expect("failed to extract temperature from split-ted fields");
        let station = fields
            .next()
            .expect("failed to extract station from split-ted fields");
        process_line((station, temperature), &mut stats);
    }

    // mmap is automatically unmapped when it goes out of scope (see mmap_file docs)
    Stats {
        _mmap: mmap,
        statistics: stats,
    }
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
/// # Safety and Soundness
/// ⚠️ **Important**: The returned `&[u8]` does *not* correctly encode
/// the lifetime of the underlying mapping in Rust’s type system.
///
/// - The slice is valid for as long as the memory mapping exists
/// - The mapping is *not* tied to the lifetime of `&File`
/// - Rust cannot verify this relationship
///
/// Correct usage requires the caller to ensure:
/// - The slice does not outlive the mapping
/// - The file is not mutated while mapped
///
/// In this program, soundness is enforced by immediately storing the
/// slice inside `Stats`, which acts as a lifetime anchor.
///
/// # Panics
/// - If file metadata cannot be read
/// - If `mmap` system call fails (e.g., insufficient memory, invalid file descriptor)
///
/// # Returns
/// A byte slice (`&[u8]`) referencing the memory-mapped file contents.
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

/// Processes a single line and updates the stats map.
/// Lifetime specifiers are required because `HashMap` is **invariant**
/// over its key type when mutably borrowed.
fn process_line<'a>(
    line: (&'a [u8], &'a [u8]),
    stats: &mut HashMap<&'a [u8], (f64, f64, usize, f64)>,
) {
    let (station, temperature) = line; // avoid utf-8 parsing except for temperature
    // SAFETY: 1BRC README.md promised valid utf-8 string characters
    let temperature = unsafe { str::from_utf8_unchecked(temperature) }
        .parse::<f64>()
        .expect("Could not parse temperature");

    // Get or insert default value for the station
    let entry = stats
        .entry(station)
        .or_insert((f64::MAX, 0_f64, 0usize, f64::MIN));

    // Update the min, sum, count, and max values for the station
    entry.0 = entry.0.min(temperature); // min
    entry.1 += temperature; // running sum
    entry.2 += 1; // count
    entry.3 = entry.3.max(temperature); // max
}

/// Formats the statistics into the required output format.
fn format_output(stats: HashMap<&[u8], (f64, f64, usize, f64)>) -> String {
    // We can;
    // a) sort all the keys,
    // b) move them into BTreeMap
    // we'll go with a
    let mut output = String::from("{");
    let stats = BTreeMap::from_iter(stats.into_iter().map(|(k, v)| {
        // SAFETY: 1BRC README.md promised valid utf-8 string characters
        //note: the key is already a reference and thus, no need for it
        // to be a String.
        (unsafe { str::from_utf8_unchecked(k) }, v)
    }));
    let mut stats = stats.iter().peekable();

    while let Some((station, (min, sum, count, max))) = stats.next() {
        let mean = sum / (*count as f64);
        output.push_str(&format!("{}={:.1}/{:.1}/{:.1}", station, min, mean, max));

        // Add comma separator if there are more items to come
        if stats.peek().is_some() {
            output.push_str(", ");
        }
    }

    output.push('}');
    output
}
