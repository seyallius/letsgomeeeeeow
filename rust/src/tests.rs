use super::*;
use crate::station::StationStats;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

// -------------------------------------------- Unit Tests --------------------------------------------

#[test]
fn test_mmap_file_small_content() {
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
    let content: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(&content)
        .expect("Failed to write to temp file");
    file.flush().expect("Failed to flush");

    let mmap = mmap_file(&file.as_file());

    assert_eq!(mmap.len(), content.len());
    assert_eq!(mmap[0], content[0]);
    assert_eq!(mmap[5000], content[5000]);
    assert_eq!(mmap[9999], content[9999]);
}

#[test]
fn test_line_parsing_with_mmap_data() {
    let file = create_test_file("Station1;10.5\nStation2;-3.2\n\nStation3;0.0\n");

    let mmap = mmap_file(&file.as_file());
    let lines: Vec<&[u8]> = mmap.split(|&byte| byte == b'\n').collect();

    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], b"Station1;10.5");
    assert_eq!(lines[1], b"Station2;-3.2");
    assert_eq!(lines[2], b""); // Empty line
    assert_eq!(lines[3], b"Station3;0.0");
    assert_eq!(lines[4], b""); // Trailing newline creates empty segment
}

#[test]
fn test_process_line_single_entry() {
    let mut stats =
        HashMap::<Vec<u8>, StationStats, _>::with_capacity_and_hasher(100, DumbHasherBuilder);
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);

    assert_eq!(stats.len(), 1);
    assert!(stats.contains_key("Hamburg".as_bytes()));

    let s = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(s.min, 120));
    assert!(approx_eq_i16(s.sum.try_into().expect("sum fits i16"), 120));
    assert_eq!(s.count, 1);
    assert!(approx_eq_i16(s.max, 120));
}

#[test]
fn test_process_line_multiple_same_station() {
    let mut stats =
        HashMap::<Vec<u8>, StationStats, _>::with_capacity_and_hasher(100, DumbHasherBuilder);
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;15.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;9.0"), &mut stats);

    assert_eq!(stats.len(), 1);

    let s = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(s.min, 90)); // 9.0 * 10
    assert!(approx_eq_i16(s.sum.try_into().expect("sum fits i16"), 360)); // (12 + 15 + 9) * 10
    assert_eq!(s.count, 3);
    assert!(approx_eq_i16(s.max, 150));
}

#[test]
fn test_process_line_multiple_stations() {
    let mut stats =
        HashMap::<Vec<u8>, StationStats, _>::with_capacity_and_hasher(100, DumbHasherBuilder);
    process_line(parse_input_to_tuple("Hamburg;12.0"), &mut stats);
    process_line(parse_input_to_tuple("Berlin;20.0"), &mut stats);
    process_line(parse_input_to_tuple("Hamburg;8.0"), &mut stats);

    assert_eq!(stats.len(), 2);

    let h = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(h.min, 80));
    assert!(approx_eq_i16(h.sum.try_into().expect("sum fits i16"), 200));
    assert_eq!(h.count, 2);
    assert!(approx_eq_i16(h.max, 120));

    let b = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq_i16(b.min, 200));
    assert!(approx_eq_i16(b.sum.try_into().expect("sum fits i16"), 200));
    assert_eq!(b.count, 1);
    assert!(approx_eq_i16(b.max, 200));
}

#[test]
fn test_process_line_negative_temperatures() {
    let mut stats =
        HashMap::<Vec<u8>, StationStats, _>::with_capacity_and_hasher(100, DumbHasherBuilder);
    process_line(parse_input_to_tuple("Oslo;-5.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-10.0"), &mut stats);
    process_line(parse_input_to_tuple("Oslo;-2.0"), &mut stats);

    let s = stats.get("Oslo".as_bytes()).unwrap();
    assert!(approx_eq_i16(s.min, -100)); // -10.0 * 10
    assert!(approx_eq_i16(s.sum.try_into().expect("sum fits i16"), -170)); // -17.0 * 10
    assert_eq!(s.count, 3);
    assert!(approx_eq_i16(s.max, -20)); // -2.0 * 10
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
    let mut stats = BTreeMap::new();
    stats.insert(
        "Hamburg".to_string(),
        StationStats {
            min: 90,
            sum: 360,
            count: 3,
            max: 150,
        },
    );

    let output = format_output(stats);
    assert_eq!(output, "{Hamburg=9.0/12.0/15.0}");
}

#[test]
fn test_format_output_multiple_stations_alphabetical() {
    let mut stats = BTreeMap::new();
    stats.insert(
        "Hamburg".to_string(),
        StationStats {
            min: 50,
            sum: 300,
            count: 3,
            max: 150,
        },
    );
    stats.insert(
        "Berlin".to_string(),
        StationStats {
            min: 100,
            sum: 450,
            count: 3,
            max: 200,
        },
    );
    stats.insert(
        "Copenhagen".to_string(),
        StationStats {
            min: 0,
            sum: 150,
            count: 3,
            max: 100,
        },
    );

    let output = format_output(stats);
    assert_eq!(
        output,
        "{Berlin=10.0/15.0/20.0, Copenhagen=0.0/5.0/10.0, Hamburg=5.0/10.0/15.0}"
    );
}

#[test]
fn test_format_output_decimal_precision() {
    let mut stats = BTreeMap::new();
    stats.insert(
        "Tokyo".to_string(),
        StationStats {
            min: 248,
            sum: 766,
            count: 3,
            max: 263,
        },
    );

    let output = format_output(stats);
    assert_eq!(output, "{Tokyo=24.8/25.5/26.3}");
}

#[test]
fn test_format_output_empty() {
    let stats = BTreeMap::<String, StationStats>::new();
    let output = format_output(stats);
    assert_eq!(output, "{}");
}

// -------------------------------------------- Integration Tests --------------------------------------------

#[test]
fn test_process_file_integration() {
    let data = "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n";
    let file = create_test_file(data);
    let mmap = mmap_file(&file.as_file());

    let stats = process_file(mmap);

    assert_eq!(stats.len(), 2);

    let h = stats.get("Hamburg".as_bytes()).unwrap();
    assert!(approx_eq_i16(h.min, 80));
    assert!(approx_eq_i16(h.sum.try_into().expect("sum fits i16"), 200));
    assert_eq!(h.count, 2);
    assert!(approx_eq_i16(h.max, 120));

    let b = stats.get("Berlin".as_bytes()).unwrap();
    assert!(approx_eq_i16(b.min, 200));
    assert!(approx_eq_i16(b.sum.try_into().expect("sum fits i16"), 450));
    assert_eq!(b.count, 2);
    assert!(approx_eq_i16(b.max, 250));
}

#[test]
fn test_process_file_with_mmap_integration() {
    let data = "A;1.0\nB;2.0\nC;3.0\n";
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(data.as_bytes())
        .expect("Failed to write to temp file");
    let mmap = mmap_file(&file.as_file());

    let stats = process_file(mmap);

    assert_eq!(stats.len(), 3);
    assert!(stats.contains_key("A".as_bytes()));
    assert!(stats.contains_key("B".as_bytes()));
    assert!(stats.contains_key("C".as_bytes()));
}

#[test]
fn test_full_pipeline() {
    let data = "Hamburg;12.0\nBerlin;20.0\nHamburg;8.0\nBerlin;25.0\n";
    let file = create_test_file(data);
    let mmap = mmap_file(&file.as_file());

    let stats = process_file(mmap);
    let output = format_output(BTreeMap::from_iter(
        stats
            .into_iter()
            .map(|(k, v)| (unsafe { String::from_utf8_unchecked(k) }, v)),
    ));

    assert_eq!(output, "{Berlin=20.0/22.5/25.0, Hamburg=8.0/10.0/12.0}");
}

#[test]
fn test_full_pipeline_with_negatives() {
    let data = "Oslo;-5.0\nOslo;-10.0\nOslo;-2.0\n";
    let file = create_test_file(data);
    let mmap = mmap_file(&file.as_file());

    let stats = process_file(mmap);
    let output = format_output(BTreeMap::from_iter(
        stats
            .into_iter()
            .map(|(k, v)| (unsafe { String::from_utf8_unchecked(k) }, v)),
    ));

    assert_eq!(output, "{Oslo=-10.0/-5.7/-2.0}");
}

// -------------------------------------------- Test Helper Functions --------------------------------------------

fn create_test_file(data: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().expect("Failed to create temp file");
    file.write_all(data.as_bytes())
        .expect("Failed to write to temp file");
    file
}

fn approx_eq_i16(a: i16, b: i16) -> bool {
    (a - b).abs() <= 1
}

fn parse_input_to_tuple(input: &str) -> (&[u8], &[u8]) {
    let (city, temp) = input.split_once(';').expect("Invalid input format");
    (city.as_bytes(), temp.as_bytes())
}
