use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::{env, io, ptr, slice};

#[cfg(test)]
mod tests;

const DEFAULT_FILE_PATH: &str = "../measurements.txt";

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

/// Processes a file and returns the statistics for all stations.
fn process_file(file_path: &str) -> HashMap<Vec<u8>, (f64, f64, usize, f64)> {
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Could not open {} file", file_path));

    //TODO(key): maybe make the key &[u8], but measure since we'll be breaking MADV_SEQUENTIAL
    // See 44c7b658 for &[u8] key.
    let mut stats = HashMap::<Vec<u8>, (f64, f64, usize, f64)>::new();

    //note: We know we're going to read the whole file, so buffered reading isn't optimal.
    // Memory mapping tells the kernel to make the file accessible as memory.
    let mmap = mmap_file(&file);

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

/// Processes a single line and updates the stats map.
fn process_line(line: (&[u8], &[u8]), stats: &mut HashMap<Vec<u8>, (f64, f64, usize, f64)>) {
    let (station, temperature) = line; // avoid utf-8 parsing except for temperature
    // SAFETY: 1BRC README.md promised valid utf-8 string characters
    let temperature = unsafe { str::from_utf8_unchecked(temperature) }
        .parse::<f64>()
        .expect("Could not parse temperature");

    // Get or insert default value for the station
    let entry = match stats.get_mut(station) {
        Some(existing_stats) => existing_stats,
        None => stats
            .entry(station.to_vec())
            .or_insert((f64::MAX, 0_f64, 0usize, f64::MIN)),
    };

    // Update the min, sum, count, and max values for the station
    entry.0 = entry.0.min(temperature); // min
    entry.1 += temperature; // running sum
    entry.2 += 1; // count
    entry.3 = entry.3.max(temperature); // max
}

/// Formats the statistics into the required output format.
fn format_output(stats: HashMap<Vec<u8>, (f64, f64, usize, f64)>) -> String {
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
