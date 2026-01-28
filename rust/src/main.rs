use std::collections::{BTreeMap, HashMap};
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
    println!("{output}");
    println!();
}

// -------------------------------------------- Helper Functions --------------------------------------------

/// Processes a file and returns the statistics for all stations.
fn process_file(file_path: &str) -> HashMap<String, (f64, f64, usize, f64)> {
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Could not open {} file", file_path));
    let file = BufReader::new(file);

    let mut stats = HashMap::<String, (f64, f64, usize, f64)>::new();

    for line in file.lines() {
        let line = line.expect("Could not read line from file");
        process_line(&line, &mut stats);
    }

    stats
}

/// Processes a single line and updates the stats map.
fn process_line(line: &str, stats: &mut HashMap<String, (f64, f64, usize, f64)>) {
    let (station, temperature) = line.split_once(';').expect("Could not parse line");
    let temperature = temperature
        .parse::<f64>()
        .expect("Could not parse temperature");

    // Get or insert default value for the station
    let entry = match stats.get_mut(station) {
        Some(existing_stats) => existing_stats,
        None => stats
            .entry(station.to_string())
            .or_insert((f64::MAX, 0_f64, 0usize, f64::MIN)),
    };

    // Update the min, sum, count, and max values for the station
    entry.0 = entry.0.min(temperature); // min
    entry.1 += temperature; // running sum
    entry.2 += 1; // count
    entry.3 = entry.3.max(temperature); // max
}

/// Formats the statistics into the required output format.
fn format_output(stats: &HashMap<String, (f64, f64, usize, f64)>) -> String {
    // We can;
    // a) sort all the keys,
    // b) move them into BTreeMap
    // we'll go with a
    let mut output = String::from("{");
    let stats = BTreeMap::from_iter(stats);
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
