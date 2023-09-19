mod heart_rate_drift;

use clap::Parser;
use heart_rate_drift::HeartRateDriftError;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Filepath. Relative or absolute should work
    filepath: String,
}

fn main() -> Result<(), HeartRateDriftError> {
    let args = Args::parse();
    Ok(())
}
