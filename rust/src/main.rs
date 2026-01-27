use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
    let output = format_output(&stats);
    print!("{output}");
}

// -------------------------------------------- Helper Functions --------------------------------------------

/// Processes a file and returns the statistics for all stations.
fn process_file(file_path: &str) -> BTreeMap<String, (f64, f64, usize, f64)> {
    let file = File::open(file_path).expect(&format!("Could not open {} file", file_path));
    let file = BufReader::new(file);

    let mut stats = BTreeMap::<String, (f64, f64, usize, f64)>::new();

    for line in file.lines() {
        let line = line.expect("Could not read line from file");
        process_line(&line, &mut stats);
    }

    stats
}

/// Processes a single line and updates the stats map.
fn process_line(line: &str, stats: &mut BTreeMap<String, (f64, f64, usize, f64)>) {
    let (station, temperature) = line.split_once(';').expect("Could not parse line");
    let temperature = temperature
        .parse::<f64>()
        .expect("Could not parse temperature");

    // Get or insert default value for the station
    let entry = stats
        .entry(station.to_string())
        .or_insert((f64::MAX, 0_f64, 0usize, f64::MIN));

    // Update the min, sum, count, and max values for the station
    entry.0 = entry.0.min(temperature); // min
    entry.1 += temperature; // running sum
    entry.2 += 1; // count
    entry.3 = entry.3.max(temperature); // max
}

/// Formats the statistics into the required output format.
fn format_output(stats: &BTreeMap<String, (f64, f64, usize, f64)>) -> String {
    let mut output = String::from("{");
    let mut stats_iter = stats.iter().peekable();

    while let Some((station, (min, sum, count, max))) = stats_iter.next() {
        let mean = sum / (*count as f64);
        output.push_str(&format!("{}={:.1}/{:.1}/{:.1}", station, min, mean, max));

        // Add comma separator if there are more items to come
        if stats_iter.peek().is_some() {
            output.push_str(", ");
        }
    }

    output.push('}');
    output
}
