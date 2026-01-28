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
    let output = format_output(stats);
    println!("{output}");
    println!();
}

// -------------------------------------------- Helper Functions --------------------------------------------

/// Processes a file and returns the statistics for all stations.
fn process_file(file_path: &str) -> HashMap<Vec<u8>, (f64, f64, usize, f64)> {
    let file =
        File::open(file_path).unwrap_or_else(|_| panic!("Could not open {} file", file_path));
    let file = BufReader::new(file);

    let mut stats = HashMap::<Vec<u8>, (f64, f64, usize, f64)>::new();

    for line in file.split(b'\n') {
        let line = line.unwrap();
        let mut fields = line.rsplitn(2, |char| *char == b';');
        let temperature = fields
            .next()
            .expect("failed to extract temperature from split-ted fields");
        let station = fields
            .next()
            .expect("failed to extract station from split-ted fields");
        process_line((station, temperature), &mut stats);
    }

    stats
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
