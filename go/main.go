package main

import (
	"fmt"
	"math"
	"os"
	"sort"
	"strconv"
	"strings"
	"syscall"
)

const defaultFilePath = "../measurements.txt"

func main() {
	filePath := defaultFilePath
	if len(os.Args) > 1 {
		filePath = os.Args[1]
	}

	stats, err := processFile(filePath)
	if err != nil {
		panic(err)
	}

	output := formatOutput(stats)
	fmt.Println(output)
	fmt.Println()
}

// -------------------------------------------- Helper Functions --------------------------------------------

// processFile reads a file and returns the statistics for all stations.
func processFile(filePath string) (map[string][4]float64, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, fmt.Errorf("could not open file: %w", err)
	}
	defer func(file *os.File) {
		if err = file.Close(); err != nil {
			panic(err)
		}
	}(file)

	stats := make(map[string][4]float64)

	//note: We know we're going to read the whole file, so buffered reading isn't optimal.
	// Memory mapping tells the kernel to make the file accessible as memory.
	mmap := mmapFile(file)
	defer func() {
		if err = syscall.Munmap(mmap); err != nil {
			panic(fmt.Sprintf("could not unmap memory: %v", err))
		}
	}()

	start := 0
	for i, b := range mmap {
		if b == '\n' {
			if i > start {
				line := string(mmap[start:i]) // Extract the substring from where we started to just before the newline
				if err = processLine(line, stats); err != nil {
					return nil, err
				}
			}
			start = i + 1 // Move start position to after the newline for next iteration
		}
	}
	// Process the last line if it doesn't end with newline
	if start < len(mmap) {
		line := string(mmap[start:])
		if len(line) > 0 {
			if err = processLine(line, stats); err != nil {
				return nil, err
			}
		}
	}

	return stats, nil
}

// mmapFile Memory-map a file into read-only byte slice using `syscall.Mmap`.
//
// This function creates a read-only memory mapping of the entire file,
// allowing direct byte access without copying data into userspace buffers.
// The mapping is backed by the file on disk and shares memory with other
// processes mapping the same file (`MAP_SHARED`).
//
// # Performance Characteristics
// - **Zero-copy**: Data is accessed directly from kernel page cache
// - **Lazy loading**: Pages are loaded on-demand (demand paging)
// - **Efficient random access**: Constant-time O(1) access to any byte offset
// - **Kernel-managed caching**: OS handles page cache automatically
//
// # Safety
//   - The returned slice is valid while the mapping exists i.e., until the file is closed.
//   - **IMPORTANT**: The slice lifetime is tied to the underlying mapping,
//     not the `File` parameter. This function's signature is misleading.
//   - The caller must ensure the file is not mutated while mapped (undefined behavior)
//   - The mapping is automatically unmapped when the slice goes out of scope
//     (via the OS when process exits, but Rust doesn't track this lifetime)
//
// # Panics
// - If file metadata cannot be read
// - If `mmap` system call fails (e.g., insufficient memory, invalid file descriptor)
//
// A byte slice (`[]byte`) referencing the memory-mapped file contents.
func mmapFile(file *os.File) []byte {
	// Get file info for memory mapping
	info, err := file.Stat()
	if err != nil {
		panic(fmt.Sprintf("could not get file info: %v", err))
	}
	fileSize := int(info.Size())

	// Memory map the file
	const OFFSET = 0
	data, err := syscall.Mmap(
		int(file.Fd()),     // File descriptor to map
		OFFSET,             // Offset of where we want to read from - Start mapping from beginning of file
		fileSize,           // Len of file - How many bytes to map
		syscall.PROT_READ,  // Memory protection: read-only
		syscall.MAP_SHARED, // Changes visible to other processes & persisted to file
	)
	if err != nil {
		panic(fmt.Sprintf("could not memory map file: %v", err))
	}

	//note: advise os on how this memory map will be accessed.
	// We're telling the kernel that when we read from a byte
	// offset, we're going to be reading in a sequential order,
	// so feel free to read ahead more (huge ass more) in advance.
	if err = syscall.Madvise(data, syscall.MADV_SEQUENTIAL); err != nil {
		panic(fmt.Sprintf("could not advise os on how this memory map will be accessed: %v", err))
	}

	return data
}

// processLine parses a single line and updates the stats map.
func processLine(line string, stats map[string][4]float64) error {
	lastSemicolon := strings.LastIndex(line, ";")
	if lastSemicolon == -1 {
		panic(fmt.Sprintf("could not parse line: %s", line))
	}

	station := line[:lastSemicolon]
	temperatureStr := line[lastSemicolon+1:]

	temperature, err := strconv.ParseFloat(temperatureStr, 64)
	if err != nil {
		panic(fmt.Sprintf("could not parse temperature: %v", err))
	}

	// Get or create the tuple this station [min, sum, count, max]
	tup, exists := stats[station]
	if !exists {
		// Initialize with default values (min=MAX, sum=0, count=0, max=MIN)
		tup = [4]float64{
			float64(^uint(0) >> 1),  // min
			0.0,                     // sum
			0.0,                     // count
			-float64(^uint(0) >> 1), // max
		}
		stats[station] = tup
	}

	// Update the min, sum, count, and max values for the station
	tup[0] = math.Min(tup[0], temperature) // min
	tup[1] += temperature                  // sum
	tup[2] += 1.0                          // count
	tup[3] = math.Max(tup[3], temperature) // max

	stats[station] = tup // <-- put the updated tup back in map

	return nil
}

// formatOutput formats the statistics into the required output format.
func formatOutput(stats map[string][4]float64) string {
	stations := make([]string, 0, len(stats))
	for station := range stats {
		stations = append(stations, station)
	}
	sort.Strings(stations)

	var output strings.Builder
	output.WriteString("{")

	for i, station := range stations {
		tup := stats[station]
		minn := tup[0]
		sum := tup[1]
		count := tup[2]
		maxx := tup[3]
		mean := sum / count

		output.WriteString(fmt.Sprintf("%s=%.1f/%.1f/%.1f", station, minn, mean, maxx))

		if i < len(stations)-1 {
			output.WriteString(", ")
		}
	}

	output.WriteString("}")
	return output.String()
}
