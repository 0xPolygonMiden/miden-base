use clap::Parser;
use std::{
    fs::{read_to_string, write, File},
    io::Write,
    path::Path,
};

mod benchmarks;
use benchmarks::*;

mod utils;

/// Root CLI struct
#[derive(Debug, Parser)]
#[clap(
    name = "Benchmark",
    about = "Benchmark execution CLI",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    bench_program: Benchmark,
}

/// CLI actions
#[derive(Debug, Parser)]
pub enum Benchmark {
    Simple,
    P2ID,
    All,
}

impl core::fmt::Display for Benchmark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Benchmark::Simple => write!(f, "simple"),
            Benchmark::P2ID => write!(f, "p2id"),
            Benchmark::All => write!(f, "all"),
        }
    }
}

/// CLI entry point
impl Cli {
    pub fn execute(&self) -> Result<(), String> {
        match &self.bench_program {
            Benchmark::Simple => {
                let path = Path::new("benchmarks/src/results/bench_simple.json");
                let mut file = File::create(path).map_err(|e| e.to_string())?;
                file.write_all(b"{}").map_err(|e| e.to_string())?;
                benchmark_default_tx(path)
            },
            Benchmark::P2ID => {
                let path = Path::new("benchmarks/src/results/bench_p2id.json");
                let mut file = File::create(path).map_err(|e| e.to_string())?;
                file.write_all(b"{}").map_err(|e| e.to_string())?;
                benchmark_p2id(path)
            },
            Benchmark::All => {
                let path = Path::new("benchmarks/src/results/bench_all.json");
                let mut file = File::create(path).map_err(|e| e.to_string())?;
                file.write_all(b"{}").map_err(|e| e.to_string())?;
                benchmark_all(path)
            },
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
