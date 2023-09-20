mod heart_rate_drift;

use anyhow::Result;
use clap::Parser;
use heart_rate_drift::{combine_hr_with_time, HeartRateDrift};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filepath. Relative or absolute should work
    filepath: String,
}

#[derive(Debug, Deserialize)]
struct HeartRates {
    data: Vec<i16>,
}

#[derive(Debug, Deserialize)]
struct Times {
    data: Vec<i16>,
}

#[derive(Debug, Deserialize)]
struct Activity {
    heartrate: HeartRates,
    time: Times,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let path = Path::new(args.filepath.as_str());
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let u: Activity = serde_json::from_reader(reader)?;

    let combined = combine_hr_with_time(u.heartrate.data.as_slice(), u.time.data.as_slice());

    print!("Heart rate drift is {}", combined.heart_rate_drift()?);

    Ok(())
}
