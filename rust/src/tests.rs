use super::*;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

// -------------------------------------------- Unit Tests --------------------------------------------

#[test]
fn test_mmap_file_small_content() {
    // Test with small content
    let content = b"Hello, mmap!";
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content)
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush");

    let mmap = mmap_file(&file.as_file());

    assert_eq!(mmap.len(), content.len());
    assert_eq!(mmap, content);
}

#[test]
fn test_mmap_file_unicode_content() {
    // Test with Unicode (still valid UTF-8)
    let content = "Hamburg;12.5\n北京;-3.7\n東京;25.0\n".as_bytes();
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(content)
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush");

    let mmap = mmap_file(&file.as_file());

    assert_eq!(mmap.len(), content.len());
    assert_eq!(mmap, content);
}

#[test]
fn test_mmap_file_large_content() {
    // Test with larger content (multiple pages)
    let content: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&content)
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush");

    let mmap = mmap_file(&file.as_file());

    assert_eq!(mmap.len(), content.len());
    // Check first, middle, and last bytes
    assert_eq!(mmap[0], content[0]);
    assert_eq!(mmap[5000], content[5000]);
    assert_eq!(mmap[9999], content[9999]);
}

// Test line splitting behavior with mmap
#[test]
fn test_line_parsing_with_mmap_data() {
    let file = create_test_file("Station1;10.5\nStation2;-3.2\n\nStation3;0.0\n");

    let mmap = mmap_file(&file.as_file());
    let lines: Vec<&[u8]> = mmap.split(|&byte| byte == b'\n').collect();

    // The data "Station1;10.5\nStation2;-3.2\n\nStation3;0.0\n" splits into:
    // 1. "Station1;10.5"
    // 2. "Station2;-3.2"
    // 3. "" (empty line)
    // 4. "Station3;0.0"
    // 5. "" (trailing newline)
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], b"Station1;10.5");
    assert_eq!(lines[1], b"Station2;-3.2");
    assert_eq!(lines[2], b""); // Empty line
    assert_eq!(lines[3], b"Station3;0.0");
    assert_eq!(lines[3], b"Station3;0.0");
    assert_eq!(lines[4], b""); // Trailing newline creates empty segment
}

#[test]
fn test_process_line_single_entry() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);

    assert_eq!(stats.len(), 1);
    assert!(stats.contains_key("Hamburg".as_bytes()));

    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, 120));
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        120
    ));
    assert_eq!(*count, 1);
    assert!(approx_eq_i16(*max, 120));
}

#[test]
fn test_process_line_multiple_same_station() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;15.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;9.0"), &mut stats);

    assert_eq!(stats.len(), 1);

    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, 90)); // 9.0 * 10
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        360
    )); // 12 + 15 + 9 = 36, *10 = 360
    assert_eq!(*count, 3);
    assert!(approx_eq_i16(*max, 150));
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
    assert!(approx_eq_i16(*min, 80)); // 8.0 * 10
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        200
    )); // 12.0 + 8.0 = 20.0, *10 = 200
    assert_eq!(*count, 2);
    assert!(approx_eq_i16(*max, 120));

    let (min, sum, count, max) = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, 200));
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        200
    ));
    assert_eq!(*count, 1);
    assert!(approx_eq_i16(*max, 200));
}

#[test]
fn test_process_line_negative_temperatures() {
    let mut stats = HashMap::new();
    process_line(parse_input_to_tuple("Oslo;-5.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-10.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-2.0"), &mut stats);

    let (min, sum, count, max) = stats.get("Oslo".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, -100)); // -10.0 * 10
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        -170
    )); // -17.0 * 10
    assert_eq!(*count, 3);
    assert!(approx_eq_i16(*max, -20)); // -2.0 * 10
}

#[test]
fn test_parse_temperature_positive_temperature() {
    assert_eq!(parse_temperature(b"12.3"), 123);
    assert_eq!(parse_temperature(b"0.1"), 1);
    assert_eq!(parse_temperature(b"99.9"), 999);
}

#[test]
fn test_parse_temperature_negative_temperature() {
    assert_eq!(parse_temperature(b"-1.0"), -10);
    assert_eq!(parse_temperature(b"-4.7"), -47);
    assert_eq!(parse_temperature(b"-99.9"), -999);
}

#[test]
fn test_parse_temperature_zero() {
    assert_eq!(parse_temperature(b"0.0"), 0);
    assert_eq!(parse_temperature(b"-0.0"), 0);
}

#[test]
fn test_parse_temperature_single_digit_before_decimal() {
    assert_eq!(parse_temperature(b"5.5"), 55);
    assert_eq!(parse_temperature(b"-5.5"), -55);
}

#[test]
fn test_format_output_single_station() {
    let mut stats = HashMap::<Vec<u8>, (i16, i64, usize, i16)>::new();
    stats.insert("Hamburg".as_bytes().to_vec(), (90, 360, 3, 150)); // 9.0, 36.0, 15.0 in tenths

    let output = format_output(stats);
    assert_eq!(output, "{Hamburg=9.0/12.0/15.0}");
}

#[test]
fn test_format_output_multiple_stations_alphabetical() {
    let mut stats = HashMap::<Vec<u8>, (i16, i64, usize, i16)>::new();
    stats.insert("Hamburg".as_bytes().to_vec(), (50, 300, 3, 150)); // 5.0, 30.0, 15.0 in tenths
    stats.insert("Berlin".as_bytes().to_vec(), (100, 450, 3, 200)); // 10.0, 45.0, 20.0 in tenths
    stats.insert("Copenhagen".as_bytes().to_vec(), (0, 150, 3, 100)); // 0.0, 15.0, 10.0 in tenths

    let output = format_output(stats);
    // BTreeMap in format_output automatically sorts keys alphabetically
    assert_eq!(
        output,
        "{Berlin=10.0/15.0/20.0, Copenhagen=0.0/5.0/10.0, Hamburg=5.0/10.0/15.0}"
    );
}

#[test]
fn test_format_output_decimal_precision() {
    let mut stats = HashMap::<Vec<u8>, (i16, i64, usize, i16)>::new();
    // sum=766, count=3, mean should be 255 (in tenths) = 25.5 (rounded to 1 decimal)
    stats.insert("Tokyo".as_bytes().to_vec(), (248, 766, 3, 263)); // 24.8, 76.6, 26.3 in tenths

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

    // Hamburg: min=8.0*10=80, sum=(12.0+8.0)*10=200, count=2, max=12.0*10=120, mean=200/2/10=10.0
    let (min, sum, count, max) = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, 80));
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        200
    ));
    assert_eq!(*count, 2);
    assert!(approx_eq_i16(*max, 120));

    // Berlin: min=20.0*10=200, sum=(20.0+25.0)*10=450, count=2, max=25.0*10=250, mean=450/2/10=22.5
    let (min, sum, count, max) = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq_i16(*min, 200));
    assert!(approx_eq_i16(
        (*sum)
            .try_into()
            .expect("should be able to convert sum to i64"),
        450
    ));
    assert_eq!(*count, 2);
    assert!(approx_eq_i16(*max, 250));
}

#[test]
fn test_process_file_with_mmap_integration() {
    // Integration test that specifically uses mmap
    let data = "A;1.0\nB;2.0\nC;3.0\n";
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(data.as_bytes())
        .expect("Failed to write to temp file");
    let file_path = file.path().to_str().unwrap();

    let stats = process_file(file_path);

    assert_eq!(stats.len(), 3);
    assert!(stats.contains_key("A".as_bytes()));
    assert!(stats.contains_key("B".as_bytes()));
    assert!(stats.contains_key("C".as_bytes()));
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

/// Checks if two i16 values are approximately equal (within 1 unit).
fn approx_eq_i16(a: i16, b: i16) -> bool {
    (a - b).abs() <= 1 // Allow tolerance of 1 for rounding differences
}

/// Parses an input string into a tuple of u8.
fn parse_input_to_tuple(input: &str) -> (&[u8], &[u8]) {
    let (city, temp) = input.split_once(';').expect("Invalid input format");
    (city.as_bytes(), temp.as_bytes())
}
