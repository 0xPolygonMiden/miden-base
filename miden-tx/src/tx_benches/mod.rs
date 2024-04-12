mod benchmarks;
use benchmarks::*;

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
    P2ID,
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        match &self.bench_program {
            Benchmarks::Simple => benchmark_default_tx(),
            Benchmarks::P2ID => todo!(),
        }
    }
}

fn main() {
    // read command-line args
    let cli = Cli::parse();

    // execute cli action
    if let Err(error) = cli.execute() {
        std::println!("{}", error);
    }
}
