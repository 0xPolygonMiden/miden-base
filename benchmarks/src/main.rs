mod benchmarks;
use benchmarks::benchmark_default_tx;

mod utils;

use clap::Parser;

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Benchmark",
    about = "Benchmark execution CLI",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    bench_program: Benchmarks,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Benchmarks {
    Simple,
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        match &self.bench_program {
            Benchmarks::Simple => benchmark_default_tx(),
        }
    }
}

fn main() {
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute() {
        println!("{}", error);
    }
}
