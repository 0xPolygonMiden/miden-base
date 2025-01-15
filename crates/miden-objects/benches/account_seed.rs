use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use miden_objects::{
    accounts::{AccountId, AccountIdVersion, AccountStorageMode, AccountType},
    Digest,
};
use rand::{Rng, SeedableRng};

/// Running this benchmark with --no-default-features will use the single-threaded account seed
/// computation.
///
/// Passing --features concurrent will use the multi-threaded account seed computation.
fn grind_account_seed(c: &mut Criterion) {
    let mut group = c.benchmark_group("grind-seed");
    // Increase measurement time (= target time) from the default 5s as suggested by criterion
    // during a run.
    group.measurement_time(Duration::from_secs(20));

    let init_seed = [
        1, 18, 222, 14, 56, 94, 222, 213, 12, 57, 86, 1, 22, 34, 187, 100, 210, 1, 18, 222, 14, 56,
        94, 43, 213, 12, 57, 86, 1, 22, 34, 187,
    ];
    // Use an rng to ensure we're starting from different seeds for each iteration.
    let mut rng = rand_xoshiro::Xoshiro256PlusPlus::from_seed(init_seed);

    group.bench_function("Grind regular on-chain account seed", |bench| {
        bench.iter(|| {
            AccountId::compute_account_seed(
                rng.gen(),
                AccountType::RegularAccountImmutableCode,
                AccountStorageMode::Public,
                AccountIdVersion::Version0,
                Digest::default(),
                Digest::default(),
                Digest::default(),
            )
        })
    });

    // Reinitialize the RNG.
    let mut rng = rand_xoshiro::Xoshiro256PlusPlus::from_seed(init_seed);
    group.bench_function("Grind fungible faucet on-chain account seed", |bench| {
        bench.iter(|| {
            AccountId::compute_account_seed(
                rng.gen(),
                AccountType::FungibleFaucet,
                AccountStorageMode::Public,
                AccountIdVersion::Version0,
                Digest::default(),
                Digest::default(),
                Digest::default(),
            )
        })
    });

    group.finish();
}

criterion_group!(account_seed, grind_account_seed);
criterion_main!(account_seed);
