use std::{fs::File, io::Write, path::PathBuf, time::Instant};

use clap::Parser;
use crypto::{hash::rpo::RpoDigest as Digest, FieldElement, Word};
use miden_mock::{
    account::DEFAULT_ACCOUNT_CODE,
    chain::{Immutable, MockChain, OnChain},
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

// TODO: update with correct faucet code
pub const DEFAULT_FAUCET_CODE: &str = "\
use.miden::sat::account

export.incr_nonce
    push.0 swap
    # => [value, 0]

    exec.account::incr_nonce
    # => [0]
end

export.set_item
    exec.account::set_item
    # => [R', V, 0, 0, 0]

    movup.8 drop movup.8 drop movup.8 drop
    # => [R', V]
end

export.set_code
    padw swapw
    # => [CODE_ROOT, 0, 0, 0, 0]

    exec.account::set_code
    # => [0, 0, 0, 0]
end

export.account_procedure_1
    push.1.2
    add
end

export.account_procedure_2
    push.2.1
    sub
end
";

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
    let mut mock_chain = MockChain::new(small_rng);
    let start = Instant::now();

    let faucet = mock_chain.build_fungible_faucet_with_seed(
        seed_to_word("3ef982ea3dca3f89179e1e86ef1c263896ef1e07e76325ce7c69825046bf75ec"),
        OnChain::Yes,
        DEFAULT_FAUCET_CODE,
        Digest::default(),
    );
    println!("Fungible faucet created {} [took: {}s]", faucet, start.elapsed().as_secs());
    mock_chain.seal_block();

    // Block 2
    // --------------------------------------------------------------------------------------------
    let amount = 100;
    let asset = mock_chain.build_fungible_asset(0, amount);

    let storage = vec![];
    let assets = vec![asset];
    let start = Instant::now();
    let account0 =
        mock_chain.build_account(DEFAULT_ACCOUNT_CODE, storage, assets, Immutable::No, OnChain::No);
    println!("Account created {} [took: {}s]", account0, start.elapsed().as_secs());

    let storage = vec![];
    let assets = vec![];
    let start = Instant::now();
    let account1 =
        mock_chain.build_account(DEFAULT_ACCOUNT_CODE, storage, assets, Immutable::No, OnChain::No);
    println!("Account created {} [took: {}s]", account1, start.elapsed().as_secs());

    let storage = vec![];
    let assets = vec![];
    let start = Instant::now();
    let account2 =
        mock_chain.build_account(DEFAULT_ACCOUNT_CODE, storage, assets, Immutable::No, OnChain::No);
    println!("Account created {} [took: {}s]", account2, start.elapsed().as_secs());

    mock_chain.seal_block();

    // Serialize
    // --------------------------------------------------------------------------------------------
    let data = postcard::to_allocvec(&mock_chain)?;

    output.write_all(&data)?;

    Ok(())
}

// HELPER FUNCTIONS
// ===============================================================================================

fn seed_to_word(seed: &str) -> Word {
    let seed_bytes = hex::decode(seed).unwrap();
    let data = unsafe { FieldElement::bytes_as_elements(&seed_bytes) }.unwrap();
    let seed: Word = [data[0], data[1], data[2], data[3]];

    seed
}

// TESTS
// ===============================================================================================

#[cfg(test)]
mod test {
    use miden_mock::chain::{from_file, MockChain};
    use rand_pcg::Pcg64;

    #[test]
    fn test_round_trip() {
        // Starting with decoding because then it is not necessary to do PoW.
        let mock: MockChain<Pcg64> = from_file("data/scenario1.bin").unwrap();
        let encoded = postcard::to_allocvec(&mock).unwrap();
        let mock2: MockChain<Pcg64> = postcard::from_bytes(&encoded).unwrap();
        let roundtrip = postcard::to_allocvec(&mock2).unwrap();

        assert_eq!(encoded, roundtrip);
    }
}
