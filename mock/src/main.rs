use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use clap::Parser;
use crypto::hash::rpo::RpoDigest as Digest;
use miden_objects::{
    builder::DEFAULT_ACCOUNT_CODE,
    mock::{Immutable, MockChain, OnChain},
};
use rand::SeedableRng;
use rand_pcg::Pcg64;

#[derive(Parser, Debug)]
struct Args {
    /// Value used to seed the PRNG.
    #[arg(long)]
    seed: Option<u64>,

    /// Output file
    output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let small_rng = match args.seed {
        Some(seed) => Pcg64::seed_from_u64(seed),
        None => Pcg64::from_entropy(),
    };

    println!(
        "Generating mock data, results will be saved to {}",
        args.output.to_str().expect("invalid file name")
    );

    let mut output = File::create(args.output)?;

    // Block 1
    // --------------------------------------------------------------------------------------------
    let mut mock_chain = MockChain::new(small_rng)?;
    let start = Instant::now();
    let faucet =
        mock_chain.new_fungible_faucet(OnChain::Yes, DEFAULT_ACCOUNT_CODE, Digest::default())?;
    println!("Fungible faucet created {} [took: {}s]", faucet, start.elapsed().as_secs());
    mock_chain.seal_block()?;

    // Block 2
    // --------------------------------------------------------------------------------------------
    let amount = 100;
    let asset = mock_chain.new_fungible_asset(0, amount)?;

    let storage = vec![];
    let assets = vec![asset];
    let start = Instant::now();
    let account0 = mock_chain.new_account(
        DEFAULT_ACCOUNT_CODE,
        storage,
        assets,
        Immutable::No,
        OnChain::No,
    )?;
    println!("Account created {} [took: {}s]", account0, start.elapsed().as_secs());

    let storage = vec![];
    let assets = vec![];
    let start = Instant::now();
    let account1 = mock_chain.new_account(
        DEFAULT_ACCOUNT_CODE,
        storage,
        assets,
        Immutable::No,
        OnChain::No,
    )?;
    println!("Account created {} [took: {}s]", account1, start.elapsed().as_secs());

    let storage = vec![];
    let assets = vec![];
    let start = Instant::now();
    let account2 = mock_chain.new_account(
        DEFAULT_ACCOUNT_CODE,
        storage,
        assets,
        Immutable::No,
        OnChain::No,
    )?;
    println!("Account created {} [took: {}s]", account2, start.elapsed().as_secs());

    mock_chain.seal_block()?;

    // Serialize
    // --------------------------------------------------------------------------------------------
    let data = postcard::to_allocvec(&mock_chain)?;

    output.write_all(&data)?;

    Ok(())
}
