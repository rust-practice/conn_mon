use std::{
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use clap::Parser;
use conn_mon::TimestampedResponse;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(author, version, about)]
/// Simple program to test deserializing the log file
struct Cli {
    /// Specifies the log file to be read in
    #[arg(value_name = "PATH")]
    log_filename: String,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    for line in read_lines(cli.log_filename)? {
        let line = line?;
        let res: TimestampedResponse = serde_json::from_str(&line)?;
        println!("{} {:?}", res.timestamp, res.response);
    }
    Ok(())
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
