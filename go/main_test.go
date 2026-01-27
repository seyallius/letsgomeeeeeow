package main

import (
	"math"
	"os"
	"testing"
)

// -------------------------------------------- Unit Tests --------------------------------------------

// TestProcessLine_SingleEntry tests processing a single line with one station.
func TestProcessLine_SingleEntry(t *testing.T) {
	stats := make(map[string][]float64)
	err := processLine("Hamburg;12.0", stats)

	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if len(stats) != 1 {
		t.Errorf("Expected 1 station, got %d", len(stats))
	}

	tup, exists := stats["Hamburg"]
	if !exists {
		t.Fatal("Hamburg not found in stats")
	}

	if !approxEqual(tup[0], 12.0) {
		t.Errorf("Expected min=12.0, got %.1f", tup[0])
	}
	if !approxEqual(tup[1], 12.0) {
		t.Errorf("Expected sum=12.0, got %.1f", tup[1])
	}
	if !approxEqual(tup[2], 1.0) {
		t.Errorf("Expected count=1, got %.1f", tup[2])
	}
	if !approxEqual(tup[3], 12.0) {
		t.Errorf("Expected max=12.0, got %.1f", tup[3])
	}
}

// TestProcessLine_MultipleSameStation tests processing multiple lines for the same station.
func TestProcessLine_MultipleSameStation(t *testing.T) {
	stats := make(map[string][]float64)

	if err := processLine("Hamburg;12.0", stats); err != nil {
	    t.Errorf("failed processing line: %v", err);
	}
	if err := processLine("Hamburg;15.0", stats); err != nil {
        t.Errorf("failed processing line: %v", err);
    }
	if err := processLine("Hamburg;9.0", stats); err != nil {
        t.Errorf("failed processing line: %v", err);
    }

	if len(stats) != 1 {
		t.Errorf("Expected 1 station, got %d", len(stats))
	}

	tup := stats["Hamburg"]
	if !approxEqual(tup[0], 9.0) {
		t.Errorf("Expected min=9.0, got %.1f", tup[0])
	}
	if !approxEqual(tup[1], 36.0) { // 12 + 15 + 9
		t.Errorf("Expected sum=36.0, got %.1f", tup[1])
	}
	if !approxEqual(tup[2], 3.0) {
		t.Errorf("Expected count=3, got %.1f", tup[2])
	}
	if !approxEqual(tup[3], 15.0) {
		t.Errorf("Expected max=15.0, got %.1f", tup[3])
	}
}

// TestProcessLine_MultipleStations tests processing multiple different stations.
func TestProcessLine_MultipleStations(t *testing.T) {
	stats := make(map[string][]float64)

	if err := processLine("Hamburg;12.0", stats); err != nil {
        t.Errorf("failed processing line: %v", err);
    }
	if err := processLine("Berlin;20.0", stats); err != nil {
        t.Errorf("failed processing line: %v", err);
    }
	if err := processLine("Hamburg;8.0", stats); err != nil {
        t.Errorf("failed processing line: %v", err);
    }

	if len(stats) != 2 {
		t.Errorf("Expected 2 stations, got %d", len(stats))
	}

	hamburg := stats["Hamburg"]
	if !approxEqual(hamburg[0], 8.0) {
		t.Errorf("Hamburg min: expected 8.0, got %.1f", hamburg[0])
	}
	if !approxEqual(hamburg[1], 20.0) {
		t.Errorf("Hamburg sum: expected 20.0, got %.1f", hamburg[1])
	}
	if !approxEqual(hamburg[2], 2.0) {
		t.Errorf("Hamburg count: expected 2, got %.1f", hamburg[2])
	}
	if !approxEqual(hamburg[3], 12.0) {
		t.Errorf("Hamburg max: expected 12.0, got %.1f", hamburg[3])
	}

	berlin := stats["Berlin"]
	if !approxEqual(berlin[0], 20.0) {
		t.Errorf("Berlin min: expected 20.0, got %.1f", berlin[0])
	}
	if !approxEqual(berlin[1], 20.0) {
		t.Errorf("Berlin sum: expected 20.0, got %.1f", berlin[1])
	}
	if !approxEqual(berlin[2], 1.0) {
		t.Errorf("Berlin count: expected 1, got %.1f", berlin[2])
	}
	if !approxEqual(berlin[3], 20.0) {
		t.Errorf("Berlin max: expected 20.0, got %.1f", berlin[3])
	}
}

// TestProcessLine_NegativeTemperatures tests processing negative temperature values.
func TestProcessLine_NegativeTemperatures(t *testing.T) {
	stats := make(map[string][]float64)

	if err := processLine("Oslo;-5.0", stats); err != nil {
	    t.Errorf("failed processing line: %v", err);
	}
	if err := processLine("Oslo;-10.0", stats); err != nil {
	    t.Errorf("failed processing line: %v", err);
	}
	if err := processLine("Oslo;-2.0", stats); err != nil {
	    t.Errorf("failed processing line: %v", err);
	}

	tup := stats["Oslo"]
	if !approxEqual(tup[0], -10.0) {
		t.Errorf("Expected min=-10.0, got %.1f", tup[0])
	}
	if !approxEqual(tup[1], -17.0) {
		t.Errorf("Expected sum=-17.0, got %.1f", tup[1])
	}
	if !approxEqual(tup[2], 3.0) {
		t.Errorf("Expected count=3, got %.1f", tup[2])
	}
	if !approxEqual(tup[3], -2.0) {
		t.Errorf("Expected max=-2.0, got %.1f", tup[3])
	}
}

// TestFormatOutput_SingleStation tests formatting output for a single station.
func TestFormatOutput_SingleStation(t *testing.T) {
	stats := map[string][]float64{
		"Hamburg": {9.0, 36.0, 3.0, 15.0},
	}

	output := formatOutput(stats)
	expected := "{Hamburg=9.0/12.0/15.0}"

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// TestFormatOutput_MultipleStationsAlphabetical tests alphabetical ordering in output.
func TestFormatOutput_MultipleStationsAlphabetical(t *testing.T) {
	stats := map[string][]float64{
		"Hamburg":    {5.0, 30.0, 3.0, 15.0},
		"Berlin":     {10.0, 45.0, 3.0, 20.0},
		"Copenhagen": {0.0, 15.0, 3.0, 10.0},
	}

	output := formatOutput(stats)
	expected := "{Berlin=10.0/15.0/20.0, Copenhagen=0.0/5.0/10.0, Hamburg=5.0/10.0/15.0}"

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// TestFormatOutput_DecimalPrecision tests decimal precision in output formatting.
func TestFormatOutput_DecimalPrecision(t *testing.T) {
	stats := map[string][]float64{
		"Tokyo": {24.8, 76.6, 3.0, 26.3}, // mean = 25.533... rounds to 25.5
	}

	output := formatOutput(stats)
	expected := "{Tokyo=24.8/25.5/26.3}"

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// TestFormatOutput_Empty tests formatting an empty stats map.
func TestFormatOutput_Empty(t *testing.T) {
	stats := make(map[string][]float64)

	output := formatOutput(stats)
	expected := "{}"

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// -------------------------------------------- Integration Tests --------------------------------------------

// TestProcessFile_Integration tests the full file processing pipeline.
func TestProcessFile_Integration(t *testing.T) {
	data := "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n"
	file := createTestFile(t, data)
	defer cleanupTestFile(t, file)

	stats, err := processFile(file.Name())
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if len(stats) != 2 {
		t.Errorf("Expected 2 stations, got %d", len(stats))
	}

	// Hamburg: min=8.0, sum=20.0, count=2, max=12.0, mean=10.0
	hamburg := stats["Hamburg"]
	if !approxEqual(hamburg[0], 8.0) {
		t.Errorf("Hamburg min: expected 8.0, got %.1f", hamburg[0])
	}
	if !approxEqual(hamburg[1], 20.0) {
		t.Errorf("Hamburg sum: expected 20.0, got %.1f", hamburg[1])
	}
	if !approxEqual(hamburg[2], 2.0) {
		t.Errorf("Hamburg count: expected 2, got %.1f", hamburg[2])
	}
	if !approxEqual(hamburg[3], 12.0) {
		t.Errorf("Hamburg max: expected 12.0, got %.1f", hamburg[3])
	}

	// Berlin: min=20.0, sum=45.0, count=2, max=25.0, mean=22.5
	berlin := stats["Berlin"]
	if !approxEqual(berlin[0], 20.0) {
		t.Errorf("Berlin min: expected 20.0, got %.1f", berlin[0])
	}
	if !approxEqual(berlin[1], 45.0) {
		t.Errorf("Berlin sum: expected 45.0, got %.1f", berlin[1])
	}
	if !approxEqual(berlin[2], 2.0) {
		t.Errorf("Berlin count: expected 2, got %.1f", berlin[2])
	}
	if !approxEqual(berlin[3], 25.0) {
		t.Errorf("Berlin max: expected 25.0, got %.1f", berlin[3])
	}
}

// TestFullPipeline tests the complete pipeline from file to formatted output.
func TestFullPipeline(t *testing.T) {
	data := "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n"
	file := createTestFile(t, data)
	defer cleanupTestFile(t, file)

	stats, err := processFile(file.Name())
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	output := formatOutput(stats)
	expected := "{Berlin=20.0/22.5/25.0, Hamburg=8.0/10.0/12.0}"

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// TestFullPipeline_WithNegatives tests the pipeline with negative temperatures.
func TestFullPipeline_WithNegatives(t *testing.T) {
	data := "Oslo;-5.0\nOslo;-10.0\nOslo;-2.0\n"
	file := createTestFile(t, data)
	defer cleanupTestFile(t, file)

	stats, err := processFile(file.Name())
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	output := formatOutput(stats)
	expected := "{Oslo=-10.0/-5.7/-2.0}" // mean = -17.0 / 3 = -5.666... rounds to -5.7

	if output != expected {
		t.Errorf("Expected %s, got %s", expected, output)
	}
}

// -------------------------------------------- Test Helper Functions --------------------------------------------

// createTestFile creates a temporary file with the given data for testing.
func createTestFile(t *testing.T, data string) *os.File {
	t.Helper()
	tmpFile, err := os.CreateTemp("", "test-measurements-*.txt")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	if _, err := tmpFile.WriteString(data); err != nil {
		t.Fatalf("Failed to write to temp file: %v", err)
	}

	// Reset file pointer to beginning
	if _, err := tmpFile.Seek(0, 0); err != nil {
		t.Fatalf("Failed to seek temp file: %v", err)
	}

	return tmpFile
}

// cleanupTestFile removes the temporary test file.
func cleanupTestFile(t *testing.T, file *os.File) {
	t.Helper()
	name := file.Name()
	file.Close()
	os.Remove(name)
}

// approxEqual checks if two float64 values are approximately equal (within 0.1).
func approxEqual(a, b float64) bool {
	return math.Abs(a-b) < 0.1
}
