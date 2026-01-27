package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
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
	fmt.Print(output)
}

// -------------------------------------------- Helper Functions --------------------------------------------

// processFile reads a file and returns the statistics for all stations.
func processFile(filePath string) (map[string][]float64, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return nil, fmt.Errorf("could not open file: %w", err)
	}
	defer file.Close()

	stats := make(map[string][]float64)

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if err := processLine(line, stats); err != nil {
			return nil, err
		}
	}

	if err := scanner.Err(); err != nil {
		return nil, fmt.Errorf("could not read line from file: %w", err)
	}

	return stats, nil
}

// processLine parses a single line and updates the stats map.
func processLine(line string, stats map[string][]float64) error {
	parts := strings.Split(line, ";")
	if len(parts) != 2 {
		return fmt.Errorf("could not parse line: %s", line)
	}

	station := parts[0]
	temperature, err := strconv.ParseFloat(parts[1], 64)
	if err != nil {
		return fmt.Errorf("could not parse temperature: %w", err)
	}

	// Get or create the tuple for this station
	tup, exists := stats[station]
	if !exists {
		// Initialize with default values (min=MAX, sum=0, count=0, max=MIN)
		tup = []float64{1<<63 - 1, 0.0, 0.0, -1 << 63}
		stats[station] = tup
	}

	// Update the min, sum, count, and max values for the station
	if temperature < tup[0] {
		tup[0] = temperature // min
	}
	tup[1] += temperature // sum
	tup[2] += 1.0         // count
	if temperature > tup[3] {
		tup[3] = temperature // max
	}

	return nil
}

// formatOutput formats the statistics into the required output format.
func formatOutput(stats map[string][]float64) string {
	stations := make([]string, 0, len(stats))
	for station := range stats {
		stations = append(stations, station)
	}
	sort.Strings(stations)

	var output strings.Builder
	output.WriteString("{")

	for i, station := range stations {
		tup := stats[station]
		min := tup[0]
		sum := tup[1]
		count := tup[2]
		max := tup[3]
		mean := sum / count

		output.WriteString(fmt.Sprintf("%s=%.1f/%.1f/%.1f", station, min, mean, max))

		if i < len(stations)-1 {
			output.WriteString(", ")
		}
	}

	output.WriteString("}")
	return output.String()
}
