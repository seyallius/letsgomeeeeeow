use super::*;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

// -------------------------------------------- Unit Tests --------------------------------------------

#[test]
fn test_process_line_single_entry() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);

    assert_eq!(stats.len(), 1);
    assert!(stats.contains_key("Hamburg".as_bytes()));

    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq(*min, 12.0));
    assert!(approx_eq(*sum, 12.0));
    assert_eq!(*count, 1);
    assert!(approx_eq(*max, 12.0));
}

#[test]
fn test_process_line_multiple_same_station() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;15.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;9.0"), &mut stats);

    assert_eq!(stats.len(), 1);

    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq(*min, 9.0));
    assert!(approx_eq(*sum, 36.0)); // 12 + 15 + 9
    assert_eq!(*count, 3);
    assert!(approx_eq(*max, 15.0));
}

#[test]
fn test_process_line_multiple_stations() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);
    process_line(parse_input_to_tuple("Berlin;20.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;8.0"), &mut stats);

    assert_eq!(stats.len(), 2);
    assert!(stats.contains_key("Hamburg".as_bytes()));
    assert!(stats.contains_key("Berlin".as_bytes()));

    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq(*min, 8.0));
    assert!(approx_eq(*sum, 20.0));
    assert_eq!(*count, 2);
    assert!(approx_eq(*max, 12.0));

    let (min, sum, count, max) = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq(*min, 20.0));
    assert!(approx_eq(*sum, 20.0));
    assert_eq!(*count, 1);
    assert!(approx_eq(*max, 20.0));
}

#[test]
fn test_process_line_negative_temperatures() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Oslo;-5.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-10.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-2.0"), &mut stats);

    let (min, sum, count, max) = stats.get("Oslo".as_bytes()).unwrap();
    assert!(approx_eq(*min, -10.0));
    assert!(approx_eq(*sum, -17.0));
    assert_eq!(*count, 3);
    assert!(approx_eq(*max, -2.0));
}

#[test]
fn test_format_output_single_station() {
    let mut stats = HashMap::<Vec<u8>, (f64, f64, usize, f64)>::new();
    stats.insert("Hamburg".as_bytes().to_vec(), (9.0, 36.0, 3, 15.0));

    let output = format_output(stats);
    assert_eq!(output, "{Hamburg=9.0/12.0/15.0}");
}

#[test]
fn test_format_output_multiple_stations_alphabetical() {
    let mut stats = HashMap::<Vec<u8>, (f64, f64, usize, f64)>::new();
    stats.insert("Hamburg".as_bytes().to_vec(), (5.0, 30.0, 3, 15.0));
    stats.insert("Berlin".as_bytes().to_vec(), (10.0, 45.0, 3, 20.0));
    stats.insert("Copenhagen".as_bytes().to_vec(), (0.0, 15.0, 3, 10.0));

    let output = format_output(stats);
    // BTreeMap in format_output automatically sorts keys alphabetically
    assert_eq!(
        output,
        "{Berlin=10.0/15.0/20.0, Copenhagen=0.0/5.0/10.0, Hamburg=5.0/10.0/15.0}"
    );
}

#[test]
fn test_format_output_decimal_precision() {
    let mut stats = HashMap::<Vec<u8>, (f64, f64, usize, f64)>::new();
    // sum=76.6, count=3, mean should be 25.5 (rounded to 1 decimal)
    stats.insert("Tokyo".as_bytes().to_vec(), (24.8, 76.6, 3, 26.3));

    let output = format_output(stats);
    assert_eq!(output, "{Tokyo=24.8/25.5/26.3}");
}

#[test]
fn test_format_output_empty() {
    let stats = HashMap::new();
    let output = format_output(stats);
    assert_eq!(output, "{}");
}

// -------------------------------------------- Integration Tests --------------------------------------------

#[test]
fn test_process_file_integration() {
    let data = "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n";
    let file = create_test_file(data);
    let file_path = file.path().to_str().unwrap();

    let stats = process_file(file_path);

    assert_eq!(stats.len(), 2);

    // Hamburg: min=8.0, sum=20.0, count=2, max=12.0, mean=10.0
    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq(*min, 8.0));
    assert!(approx_eq(*sum, 20.0));
    assert_eq!(*count, 2);
    assert!(approx_eq(*max, 12.0));

    // Berlin: min=20.0, sum=45.0, count=2, max=25.0, mean=22.5
    let (min, sum, count, max) = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq(*min, 20.0));
    assert!(approx_eq(*sum, 45.0));
    assert_eq!(*count, 2);
    assert!(approx_eq(*max, 25.0));
}

#[test]
fn test_full_pipeline() {
    let data = "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n";
    let file = create_test_file(data);
    let file_path = file.path().to_str().unwrap();

    let stats = process_file(file_path);
    let output = format_output(stats);

    assert_eq!(output, "{Berlin=20.0/22.5/25.0, Hamburg=8.0/10.0/12.0}");
}

#[test]
fn test_full_pipeline_with_negatives() {
    let data = "Oslo;-5.0\nOslo;-10.0\nOslo;-2.0\n";
    let file = create_test_file(data);
    let file_path = file.path().to_str().unwrap();

    let stats = process_file(file_path);
    let output = format_output(stats);

    // mean = -17.0 / 3 = -5.666... rounds to -5.7
    assert_eq!(output, "{Oslo=-10.0/-5.7/-2.0}");
}

// -------------------------------------------- Test Helper Functions --------------------------------------------

/// Creates a temporary file with test data for measurements.
fn create_test_file(data: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(data.as_bytes())
        .expect("Failed to write to temp file");
    file
}

/// Checks if two f64 values are approximately equal (within 0.1).
fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.1
}

/// Parses an input string into a tuple of u8.
fn parse_input_to_tuple(input: &str) -> (&[u8], &[u8]) {
    let (city, temp) = input.split_once(';').expect("Invalid input format");
    (city.as_bytes(), temp.as_bytes())
}
