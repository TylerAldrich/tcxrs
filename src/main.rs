use std::{path::Path, time::SystemTime};
use tracing::info;

use clap::Parser;
use tcxrs::display_folder_stats;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the directory to parse tcx files within
    directory: String,

    /// Name of the file to print output data into.
    #[arg(short, long, default_value = "output.txt")]
    output_file: String,

    /// Name of the file to write the chart to
    #[arg(short, long, default_value = "output-bitmap.png")]
    chart: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let start = SystemTime::now();
    if let Err(e) = display_folder_stats(
        Path::new(&args.directory),
        Path::new(&args.output_file),
        args.chart,
    )
    .await
    {
        eprintln!("{}", e);
    }
    let end = SystemTime::now();
    let duration = end.duration_since(start).unwrap();
    info!("Total time: {:?}", duration)
}
